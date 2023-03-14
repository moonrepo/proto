#![allow(clippy::borrowed_box)]

use crate::color;
use crate::config::{Config, CONFIG_NAME};
use crate::errors::ProtoError;
use crate::manifest::{Manifest, MANIFEST_NAME};
use crate::tool::Tool;
use lenient_semver::Version;
use log::{debug, trace};
use std::{env, fs, path::Path};

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

pub async fn detect_version_from_environment<'l, T: Tool<'l> + ?Sized>(
    tool: &Box<T>,
    forced_version: Option<String>,
) -> Result<String, ProtoError> {
    let mut version = forced_version;
    let env_var = format!("PROTO_{}_VERSION", tool.get_bin_name().to_uppercase());

    // Env var takes highest priority
    if version.is_none() {
        if let Ok(session_version) = env::var(&env_var) {
            debug!(
                target: "proto:detector",
                "Detected version {} from environment variable {}",
                session_version,
                env_var
            );

            version = Some(session_version);
        }
    } else {
        debug!(
            target: "proto:detector",
            "Using explicit version {} passed on the command line",
            version.as_ref().unwrap(),
        );
    }

    // Traverse upwards and attempt to detect a local version
    if let Ok(working_dir) = env::current_dir() {
        trace!(
            target: "proto:detector",
            "Attempting to find local version"
        );

        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            trace!(
                target: "proto:detector",
                "Checking in directory {}",
                color::path(dir)
            );

            // We already found a version, so exit
            if version.is_some() {
                break;
            }

            // Detect from our config file
            trace!(
                target: "proto:detector",
                "Checking proto configuration file ({})",
                CONFIG_NAME
            );

            let config = Config::load_from(dir)?;

            if let Some(local_version) = config.tools.get(tool.get_bin_name()) {
                debug!(
                    target: "proto:detector",
                    "Detected version {} from configuration file {}",
                    local_version,
                    color::path(&config.path)
                );

                version = Some(local_version.to_owned());
                break;
            }

            // Detect using the tool
            trace!(
                target: "proto:detector",
                "Detecting from the tool's ecosystem"
            );

            if let Some(eco_version) = tool.detect_version_from(dir).await? {
                debug!(
                    target: "proto:detector",
                    "Detected version {} from tool's ecosystem",
                    eco_version,
                );

                version = Some(eco_version);
                break;
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
    if version.is_none() {
        trace!(
            target: "proto:detector",
            "Attempting to find global version in manifest ({})",
            MANIFEST_NAME
        );

        let manifest = Manifest::load_for_tool(tool.get_bin_name())?;

        if let Some(global_version) = manifest.default_version {
            debug!(
                target: "proto:detector",
                "Detected global version {} from {}",
               global_version,
                color::path(&manifest.path)
            );

            version = Some(global_version);
        }
    }

    // We didn't find anything!
    match version {
        Some(ver) => Ok(ver),
        None => Err(ProtoError::Message(
            "Unable to detect an applicable version. Try setting a local or global version, or passing a command line argument.".into(),
        )),
    }
}

pub fn detect_fixed_version<P: AsRef<Path>>(
    version: &str,
    manifest_path: P,
) -> Result<Option<String>, ProtoError> {
    let version = version.replace(' ', "");
    let version_without_stars = version.replace(".*", "");
    let mut maybe_version = String::new();

    match &version[0..1] {
        "^" | "~" | ">" | "<" | "*" => {
            let req = semver::VersionReq::parse(&version)
                .map_err(|e| ProtoError::Semver(version.to_owned(), e.to_string()))?;
            let manifest = Manifest::load(manifest_path.as_ref())?;

            for installed_version in manifest.installed_versions {
                let version_inst = semver::Version::parse(&installed_version)
                    .map_err(|e| ProtoError::Semver(installed_version.to_owned(), e.to_string()))?;

                if req.matches(&version_inst) {
                    maybe_version = installed_version;
                    break;
                }
            }
        }
        "=" => {
            maybe_version = version_without_stars[1..].to_owned();
        }
        _ => {
            maybe_version = if version.contains('*') {
                version_without_stars
            } else {
                version
            };
        }
    };

    if maybe_version.is_empty() {
        return Ok(None);
    }

    let semver = Version::parse(&maybe_version)
        .map_err(|e| ProtoError::Semver(maybe_version.to_owned(), e.to_string()))?;
    let mut matched_version = semver.major.to_string();

    if semver.minor != 0 {
        matched_version = format!("{matched_version}.{}", semver.minor);

        if semver.patch != 0 {
            matched_version = format!("{matched_version}.{}", semver.patch);
        }
    }

    Ok(Some(matched_version))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    pub fn create_temp_dir() -> assert_fs::TempDir {
        assert_fs::TempDir::new().unwrap()
    }

    pub fn create_manifest(dir: &Path, manifest: Manifest) -> PathBuf {
        let manifest_path = dir.join(MANIFEST_NAME);
        let manifest_str = serde_json::to_string_pretty(&manifest).unwrap();

        dbg!(&manifest_str);

        std::fs::write(&manifest_path, manifest_str).unwrap();

        manifest_path
    }

    mod fixed_version {
        use rustc_hash::FxHashSet;

        use super::*;

        // #[test]
        // fn ignores_invalid() {
        //     assert_eq!(detect_fixed_version("unknown"), None);
        //     assert_eq!(detect_fixed_version("1 || 2"), None);
        // }

        #[test]
        fn handles_explicit() {
            let temp = create_temp_dir();

            assert_eq!(
                detect_fixed_version("1.2.3-alpha", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2.3"
            ); // ?
            assert_eq!(
                detect_fixed_version("1.2.3", temp.path()).unwrap().unwrap(),
                "1.2.3"
            );
            assert_eq!(
                detect_fixed_version("1.2.0", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("1.2", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("1.0", temp.path()).unwrap().unwrap(),
                "1"
            );
            assert_eq!(
                detect_fixed_version("1", temp.path()).unwrap().unwrap(),
                "1"
            );

            assert_eq!(
                detect_fixed_version("v1.2.3-alpha", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2.3"
            ); // ?
            assert_eq!(
                detect_fixed_version("V1.2.3", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2.3"
            );
            assert_eq!(
                detect_fixed_version("v1.2.0", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("V1.2", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("v1.0", temp.path()).unwrap().unwrap(),
                "1"
            );
            assert_eq!(
                detect_fixed_version("V1", temp.path()).unwrap().unwrap(),
                "1"
            );
        }

        #[test]
        fn handles_equals() {
            let temp = create_temp_dir();

            assert_eq!(
                detect_fixed_version("=1.2.3-alpha", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2.3"
            ); // ?
            assert_eq!(
                detect_fixed_version("=1.2.3", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2.3"
            );
            assert_eq!(
                detect_fixed_version("=1.2.0", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("=1.2", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("=1.0", temp.path()).unwrap().unwrap(),
                "1"
            );
            assert_eq!(
                detect_fixed_version("=1", temp.path()).unwrap().unwrap(),
                "1"
            );
        }

        #[test]
        fn handles_star() {
            let temp = create_temp_dir();

            assert_eq!(
                detect_fixed_version("=1.2.*", temp.path())
                    .unwrap()
                    .unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("=1.*", temp.path()).unwrap().unwrap(),
                "1"
            );
            assert_eq!(
                detect_fixed_version("1.2.*", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("1.*", temp.path()).unwrap().unwrap(),
                "1"
            );
        }

        #[test]
        fn handles_star_all() {
            let temp = create_temp_dir();

            let manifest_path = create_manifest(temp.path(), Manifest::default());

            assert_eq!(detect_fixed_version("*", manifest_path).unwrap(), None);

            let manifest_path = create_manifest(
                temp.path(),
                Manifest {
                    installed_versions: FxHashSet::from_iter(["1.2.3".into()]),
                    ..Manifest::default()
                },
            );

            assert_eq!(
                detect_fixed_version("*", manifest_path).unwrap().unwrap(),
                "1.2.3"
            );
        }

        // #[test]
        // fn handles_caret() {
        //     assert_eq!(detect_fixed_version("^1.2.3-alpha").unwrap(), "1.2.3"); // ?
        //     assert_eq!(detect_fixed_version("^1.2.3").unwrap(), "1.2.3");
        //     assert_eq!(detect_fixed_version("^1.2.0").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version("^1.2").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version("^1.0").unwrap(), "1");
        //     assert_eq!(detect_fixed_version("^1").unwrap(), "1");
        // }

        // #[test]
        // fn handles_tilde() {
        //     assert_eq!(detect_fixed_version("~1.2.3-alpha").unwrap(), "1.2.3"); // ?
        //     assert_eq!(detect_fixed_version("~1.2.3").unwrap(), "1.2.3");
        //     assert_eq!(detect_fixed_version("~1.2.0").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version("~1.2").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version("~1.0").unwrap(), "1");
        //     assert_eq!(detect_fixed_version("~1").unwrap(), "1");
        // }

        // #[test]
        // fn handles_gt() {
        //     assert_eq!(detect_fixed_version(">1.2.3-alpha").unwrap(), "1.2"); // ?
        //     assert_eq!(detect_fixed_version(">1.2.3").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version(">1.2.0").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version(">1.2").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version(">1.0").unwrap(), "1");
        //     assert_eq!(detect_fixed_version(">1").unwrap(), "1");

        //     assert_eq!(detect_fixed_version(">=1.2.3-alpha").unwrap(), "1.2.3"); // ?
        //     assert_eq!(detect_fixed_version(">=1.2.3").unwrap(), "1.2.3");
        //     assert_eq!(detect_fixed_version(">=1.2.0").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version(">=1.2").unwrap(), "1.2");
        //     assert_eq!(detect_fixed_version(">=1.0").unwrap(), "1");
        //     assert_eq!(detect_fixed_version(">=1").unwrap(), "1");
        // }

        // #[test]
        // fn handles_lt() {
        //     // THIS IS WRONG, best solution? Does this even happen?
        //     assert_eq!(detect_fixed_version("<1.2.3-alpha").unwrap(), "1.2.3"); // ?
        //     assert_eq!(detect_fixed_version("<1.2.3").unwrap(), "1.2.3");
        //     assert_eq!(detect_fixed_version("<1.2.0").unwrap(), "1.2.0");
        //     assert_eq!(detect_fixed_version("<1.2").unwrap(), "1.2.0");
        //     assert_eq!(detect_fixed_version("<1.0").unwrap(), "1.0.0");
        //     assert_eq!(detect_fixed_version("<1").unwrap(), "1.0.0");

        //     assert_eq!(detect_fixed_version("<=1.2.3-alpha").unwrap(), "1.2.3"); // ?
        //     assert_eq!(detect_fixed_version("<=1.2.3").unwrap(), "1.2.3");
        //     assert_eq!(detect_fixed_version("<=1.2.0").unwrap(), "1.2.0");
        //     assert_eq!(detect_fixed_version("<=1.2").unwrap(), "1.2.0");
        //     assert_eq!(detect_fixed_version("<=1.0").unwrap(), "1.0.0");
        //     assert_eq!(detect_fixed_version("<=1").unwrap(), "1.0.0");
        // }
    }
}
