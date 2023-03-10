use crate::errors::ProtoError;
use lenient_semver::Version;
use std::{fs, path::Path};

#[async_trait::async_trait]
pub trait Detector<'tool>: Send + Sync {
    /// Attempt to detect an applicable version from the provided working directory.
    async fn detect_version_from(&self, _working_dir: &Path) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }
}

pub fn load_version_file(path: &Path) -> Result<String, ProtoError> {
    Ok(fs::read_to_string(path)
        .map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?
        .trim()
        .to_owned())
}

pub fn get_fixed_version(version: &str) -> Option<String> {
    if version == "*" {
        return Some("latest".into());
    }

    let version = version.replace(' ', "");
    let version_without_stars = version.replace(".*", "");
    let mut explicit = false;
    let mut drop_patch = false;

    // Multiple versions, unable to parse
    if version.contains('|') {
        return None;
    }

    let semver = match &version[0..1] {
        "^" | "~" => Version::parse(&version[1..]),
        ">" => {
            if let Some(v) = version.strip_prefix(">=") {
                Version::parse(v)
            } else {
                drop_patch = true;
                Version::parse(&version[1..])
            }
        }
        "<" => {
            explicit = true;

            if let Some(v) = version.strip_prefix("<=") {
                Version::parse(v)
            } else {
                // TODO: This isn't correct
                Version::parse(&version[1..])
            }
        }
        "=" => {
            explicit = true;
            Version::parse(&version_without_stars[1..])
        }
        _ => {
            if version.contains('*') {
                Version::parse(&version_without_stars)
            } else {
                explicit = true;
                Version::parse(&version)
            }
        }
    };

    let Ok(semver) = semver else {
        return None;
    };

    let mut matched_version = semver.major.to_string();

    if semver.minor != 0 || explicit {
        matched_version = format!("{matched_version}.{}", semver.minor);

        if (semver.patch != 0 || explicit) && !drop_patch {
            matched_version = format!("{matched_version}.{}", semver.patch);
        }
    }

    Some(matched_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fixed_version {
        use super::*;

        #[test]
        fn ignores_invalid() {
            assert_eq!(get_fixed_version("unknown"), None);
            assert_eq!(get_fixed_version("1 || 2"), None);
        }

        #[test]
        fn handles_explicit() {
            assert_eq!(get_fixed_version("1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version("1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version("1.2.0").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("1.2").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("1.0").unwrap(), "1.0.0");
            assert_eq!(get_fixed_version("1").unwrap(), "1.0.0");

            assert_eq!(get_fixed_version("v1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version("V1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version("v1.2.0").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("V1.2").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("v1.0").unwrap(), "1.0.0");
            assert_eq!(get_fixed_version("V1").unwrap(), "1.0.0");
        }

        #[test]
        fn handles_equals() {
            assert_eq!(get_fixed_version("=1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version("=1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version("=1.2.0").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("=1.2").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("=1.0").unwrap(), "1.0.0");
            assert_eq!(get_fixed_version("=1").unwrap(), "1.0.0");
        }

        #[test]
        fn handles_star() {
            assert_eq!(get_fixed_version("=1.2.*").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("=1.*").unwrap(), "1.0.0");
            assert_eq!(get_fixed_version("1.2.*").unwrap(), "1.2");
            assert_eq!(get_fixed_version("1.*").unwrap(), "1");
            assert_eq!(get_fixed_version("*").unwrap(), "latest");
        }

        #[test]
        fn handles_caret() {
            assert_eq!(get_fixed_version("^1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version("^1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version("^1.2.0").unwrap(), "1.2");
            assert_eq!(get_fixed_version("^1.2").unwrap(), "1.2");
            assert_eq!(get_fixed_version("^1.0").unwrap(), "1");
            assert_eq!(get_fixed_version("^1").unwrap(), "1");
        }

        #[test]
        fn handles_tilde() {
            assert_eq!(get_fixed_version("~1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version("~1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version("~1.2.0").unwrap(), "1.2");
            assert_eq!(get_fixed_version("~1.2").unwrap(), "1.2");
            assert_eq!(get_fixed_version("~1.0").unwrap(), "1");
            assert_eq!(get_fixed_version("~1").unwrap(), "1");
        }

        #[test]
        fn handles_gt() {
            assert_eq!(get_fixed_version(">1.2.3-alpha").unwrap(), "1.2"); // ?
            assert_eq!(get_fixed_version(">1.2.3").unwrap(), "1.2");
            assert_eq!(get_fixed_version(">1.2.0").unwrap(), "1.2");
            assert_eq!(get_fixed_version(">1.2").unwrap(), "1.2");
            assert_eq!(get_fixed_version(">1.0").unwrap(), "1");
            assert_eq!(get_fixed_version(">1").unwrap(), "1");

            assert_eq!(get_fixed_version(">=1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version(">=1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version(">=1.2.0").unwrap(), "1.2");
            assert_eq!(get_fixed_version(">=1.2").unwrap(), "1.2");
            assert_eq!(get_fixed_version(">=1.0").unwrap(), "1");
            assert_eq!(get_fixed_version(">=1").unwrap(), "1");
        }

        #[test]
        fn handles_lt() {
            // THIS IS WRONG, best solution? Does this even happen?
            assert_eq!(get_fixed_version("<1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version("<1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version("<1.2.0").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("<1.2").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("<1.0").unwrap(), "1.0.0");
            assert_eq!(get_fixed_version("<1").unwrap(), "1.0.0");

            assert_eq!(get_fixed_version("<=1.2.3-alpha").unwrap(), "1.2.3"); // ?
            assert_eq!(get_fixed_version("<=1.2.3").unwrap(), "1.2.3");
            assert_eq!(get_fixed_version("<=1.2.0").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("<=1.2").unwrap(), "1.2.0");
            assert_eq!(get_fixed_version("<=1.0").unwrap(), "1.0.0");
            assert_eq!(get_fixed_version("<=1").unwrap(), "1.0.0");
        }
    }
}
