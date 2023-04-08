#![allow(clippy::borrowed_box)]

use crate::errors::ProtoError;
use crate::helpers::is_alias_name;
use crate::manifest::{Manifest, MANIFEST_NAME};
use crate::tool::Tool;
use crate::tools_config::{ToolsConfig, TOOLS_CONFIG_NAME};
use lenient_semver::Version;
use starbase_styles::color;
use starbase_utils::fs;
use std::{env, path::Path};
use tracing::{debug, trace};

#[async_trait::async_trait]
pub trait Detector<'tool>: Send + Sync {
    /// Attempt to detect an applicable version from the provided working directory.
    async fn detect_version_from(&self, _working_dir: &Path) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }
}

pub fn load_version_file(path: &Path) -> Result<String, ProtoError> {
    Ok(fs::read_file(path)?.trim().to_owned())
}

#[tracing::instrument(skip_all)]
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
                "Detected version {} from environment variable {}",
                session_version, env_var
            );

            version = Some(session_version);
        }
    } else {
        debug!(
            "Using explicit version {} passed on the command line",
            version.as_ref().unwrap(),
        );
    }

    // Traverse upwards and attempt to detect a local version
    if let Ok(working_dir) = env::current_dir() {
        trace!("Attempting to find local version");

        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            trace!("Checking in directory {}", color::path(dir));

            // We already found a version, so exit
            if version.is_some() {
                break;
            }

            // Detect from our config file
            trace!("Checking proto configuration file ({})", TOOLS_CONFIG_NAME);

            let config = ToolsConfig::load_from(dir)?;

            if let Some(local_version) = config.tools.get(tool.get_bin_name()) {
                debug!(
                    "Detected version {} from configuration file {}",
                    local_version,
                    color::path(&config.path)
                );

                version = Some(local_version.to_owned());
                break;
            }

            // Detect using the tool
            trace!("Detecting from the tool's ecosystem");

            if let Some(eco_version) = tool.detect_version_from(dir).await? {
                debug!("Detected version {} from tool's ecosystem", eco_version,);

                version = Some(eco_version);
                break;
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
    if version.is_none() {
        trace!(
            "Attempting to find global version in manifest ({})",
            MANIFEST_NAME
        );

        let manifest = Manifest::load(tool.get_manifest_path())?;

        if let Some(global_version) = manifest.default_version {
            debug!(
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

#[tracing::instrument(skip_all)]
pub fn detect_fixed_version<P: AsRef<Path>>(
    version: &str,
    manifest_path: P,
) -> Result<Option<String>, ProtoError> {
    if is_alias_name(version) {
        return Ok(None);
    }

    let version = version.replace(".*", "");
    let mut fully_qualified = false;
    let mut maybe_version = String::new();

    let mut check_manifest = |check_version: String| -> Result<bool, ProtoError> {
        let req =
            semver::VersionReq::parse(&check_version).map_err(|error| ProtoError::Semver {
                version: check_version.to_owned(),
                error,
            })?;
        let manifest = Manifest::load(manifest_path.as_ref())?;

        for installed_version in manifest.installed_versions {
            let version_inst =
                semver::Version::parse(&installed_version).map_err(|error| ProtoError::Semver {
                    version: installed_version.to_owned(),
                    error,
                })?;

            if req.matches(&version_inst) {
                fully_qualified = true;
                maybe_version = installed_version;

                return Ok(true);
            }
        }

        Ok(false)
    };

    // npm
    if version.contains("||") {
        for split_version in version.split("||") {
            if check_manifest(split_version.trim().to_owned())? {
                break;
            }
        }
    } else {
        match &version[0..1] {
            "^" | "~" | ">" | "<" | "*" => {
                check_manifest(version.clone())?;
            }
            "=" => {
                maybe_version = version[1..].to_owned();
            }
            _ => {
                maybe_version = version.clone();
            }
        };
    }

    if maybe_version.is_empty() {
        return Ok(None);
    }

    let semver = Version::parse(&maybe_version).map_err(|e| ProtoError::Message(e.to_string()))?;

    let version_parts = version.split('.').collect::<Vec<_>>();
    let mut matched_version = semver.major.to_string();

    if version_parts.get(1).is_some() || fully_qualified {
        matched_version = format!("{matched_version}.{}", semver.minor);

        if version_parts.get(2).is_some() || fully_qualified {
            matched_version = format!("{matched_version}.{}", semver.patch);
        }
    }

    Ok(Some(matched_version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    pub fn create_temp_dir() -> assert_fs::TempDir {
        assert_fs::TempDir::new().unwrap()
    }

    pub fn create_manifest(dir: &Path, manifest: Manifest) -> PathBuf {
        let manifest_path = dir.join(MANIFEST_NAME);

        starbase_utils::json::write_file(&manifest_path, &manifest, true).unwrap();

        manifest_path
    }

    mod fixed_version {
        use super::*;
        use rustc_hash::FxHashSet;

        #[test]
        fn ignores_invalid() {
            let temp = create_temp_dir();

            assert_eq!(detect_fixed_version("unknown", temp.path()).unwrap(), None);
        }

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
                "1.2.0"
            );
            assert_eq!(
                detect_fixed_version("1.2", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("1.0", temp.path()).unwrap().unwrap(),
                "1.0"
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
                "1.2.0"
            );
            assert_eq!(
                detect_fixed_version("V1.2", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("v1.0", temp.path()).unwrap().unwrap(),
                "1.0"
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
                "1.2.0"
            );
            assert_eq!(
                detect_fixed_version("=1.2", temp.path()).unwrap().unwrap(),
                "1.2"
            );
            assert_eq!(
                detect_fixed_version("=1.0", temp.path()).unwrap().unwrap(),
                "1.0"
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

        #[test]
        fn handles_caret() {
            let temp = create_temp_dir();
            let manifest_path = create_manifest(
                temp.path(),
                Manifest {
                    installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                    ..Manifest::default()
                },
            );

            assert_eq!(
                detect_fixed_version("^1.2.3-alpha", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version("^1.2.3", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version("^1.2.0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version("^1.2", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version("^1.0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version("^1", &manifest_path).unwrap().unwrap(),
                "1.5.9"
            );

            // Failures
            assert_eq!(detect_fixed_version("^1.6", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version("^2", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version("^0", &manifest_path).unwrap(), None);
        }

        #[test]
        fn handles_tilde() {
            let temp = create_temp_dir();
            let manifest_path = create_manifest(
                temp.path(),
                Manifest {
                    installed_versions: FxHashSet::from_iter(["1.2.9".into()]),
                    ..Manifest::default()
                },
            );

            assert_eq!(
                detect_fixed_version("~1.2.3-alpha", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.2.9"
            );
            assert_eq!(
                detect_fixed_version("~1.2.3", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.2.9"
            );
            assert_eq!(
                detect_fixed_version("~1.2.0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.2.9"
            );
            assert_eq!(
                detect_fixed_version("~1.2", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.2.9"
            );
            assert_eq!(
                detect_fixed_version("~1", &manifest_path).unwrap().unwrap(),
                "1.2.9"
            );

            // Failures
            assert_eq!(detect_fixed_version("~1.3", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version("~1.1", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version("~1.0", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version("~2", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version("~0", &manifest_path).unwrap(), None);
        }

        #[test]
        fn handles_gt() {
            let temp = create_temp_dir();
            let manifest_path = create_manifest(
                temp.path(),
                Manifest {
                    installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                    ..Manifest::default()
                },
            );

            assert_eq!(
                detect_fixed_version(">1.2.3-alpha", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">1.2.3", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">1.2.0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">1.2", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">1.0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">0", &manifest_path).unwrap().unwrap(),
                "1.5.9"
            );

            // Failures
            assert_eq!(detect_fixed_version(">1.6", &manifest_path).unwrap(), None);
            assert_eq!(
                detect_fixed_version(">1.5.9", &manifest_path).unwrap(),
                None
            );
            assert_eq!(detect_fixed_version(">2", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version(">1", &manifest_path).unwrap(), None);
        }

        #[test]
        fn handles_gte() {
            let temp = create_temp_dir();
            let manifest_path = create_manifest(
                temp.path(),
                Manifest {
                    installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                    ..Manifest::default()
                },
            );

            assert_eq!(
                detect_fixed_version(">=1.2.3-alpha", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">=1.2.3", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">=1.2.0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">=1.2", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">=1.0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">=1", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version(">=0", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );

            // Failures
            assert_eq!(detect_fixed_version(">1.6", &manifest_path).unwrap(), None);
            assert_eq!(detect_fixed_version(">=2", &manifest_path).unwrap(), None);
        }

        #[test]
        fn handles_multi() {
            let temp = create_temp_dir();
            let manifest_path = create_manifest(
                temp.path(),
                Manifest {
                    installed_versions: FxHashSet::from_iter(["1.5.9".into()]),
                    ..Manifest::default()
                },
            );

            assert_eq!(
                detect_fixed_version("^1.2.3 || ^2", &manifest_path)
                    .unwrap()
                    .unwrap(),
                "1.5.9"
            );
            assert_eq!(
                detect_fixed_version("^1.6 || ^2", &manifest_path).unwrap(),
                None
            );
        }
    }
}
