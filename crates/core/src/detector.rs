#![allow(clippy::borrowed_box)]

use crate::errors::ProtoError;
use crate::helpers::{is_alias_name, remove_v_prefix, remove_space_after_gtlt};
use crate::manifest::Manifest;
use crate::tool::Tool;
use crate::tools_config::ToolsConfig;
use human_sort::compare;
use lenient_semver::Version;
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
pub async fn detect_version<'l, T: Tool<'l> + ?Sized>(
    tool: &Box<T>,
    forced_version: Option<String>,
) -> Result<String, ProtoError> {
    let mut version = forced_version;
    let env_var = format!("PROTO_{}_VERSION", tool.get_id().to_uppercase());

    // Env var takes highest priority
    if version.is_none() {
        if let Ok(session_version) = env::var(&env_var) {
            debug!(
                tool = tool.get_id(),
                env_var,
                version = session_version,
                "Detected version from environment variable",
            );

            version = Some(session_version);
        } else {
            trace!(
                tool = tool.get_id(),
                "Attempting to find local version from config files"
            );
        }
    } else {
        debug!(
            tool = tool.get_id(),
            version = version.as_ref().unwrap(),
            "Using explicit version passed on the command line",
        );
    }

    // Traverse upwards and attempt to detect a local version
    if let Ok(working_dir) = env::current_dir() {
        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            // We already found a version, so exit
            if version.is_some() {
                break;
            }

            trace!(
                tool = tool.get_id(),
                dir = ?dir,
                "Checking directory",
            );

            // Detect from our config file
            let config = ToolsConfig::load_from(dir)?;

            if let Some(local_version) = config.tools.get(tool.get_id()) {
                debug!(
                    tool = tool.get_id(),
                    version = local_version,
                    file = ?config.path,
                    "Detected version from .prototools file",
                );

                version = Some(local_version.to_owned());
                break;
            }

            // Detect using the tool
            if let Some(eco_version) = tool.detect_version_from(dir).await? {
                if let Some(eco_version) =
                    expand_detected_version(&eco_version, tool.get_manifest()?)?
                {
                    debug!(
                        tool = tool.get_id(),
                        version = eco_version,
                        "Detected version from tool's ecosystem"
                    );

                    version = Some(eco_version);
                    break;
                }
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
    if version.is_none() {
        trace!(
            tool = tool.get_id(),
            "Attempting to find global version in manifest",
        );

        let manifest = tool.get_manifest()?;

        if let Some(global_version) = &manifest.default_version {
            debug!(
                tool = tool.get_id(),
                version = global_version,
                file = ?manifest.path,
                "Detected global version from manifest",
            );

            version = Some(global_version.to_owned());
        }
    }

    // We didn't find anything!
    match version {
        Some(ver) => Ok(ver),
        None => Err(ProtoError::VersionDetectFailed(tool.get_id().to_owned())),
    }
}

#[tracing::instrument(skip_all)]
pub fn expand_detected_version(
    version: &str,
    manifest: &Manifest,
) -> Result<Option<String>, ProtoError> {
    if is_alias_name(version) {
        return Ok(Some(version.to_owned()));
    }

    let version = remove_space_after_gtlt(&remove_v_prefix(&version.replace(".*", "")));
    let mut fully_qualified = false;
    let mut maybe_version = String::new();

    // Sort the installed versions in descending order, so that v20
    // is preferred over v2, and v19 for a requirement like >=15.
    let mut installed_versions = manifest
        .installed_versions
        .iter()
        .map(|v| v.to_owned())
        .collect::<Vec<String>>();

    installed_versions.sort_by(|a, d| compare(d, a));

    let mut check_manifest = |check_version: String| -> Result<bool, ProtoError> {
        let req =
            semver::VersionReq::parse(&check_version).map_err(|error| ProtoError::Semver {
                version: check_version.to_owned(),
                error,
            })?;

        for installed_version in &installed_versions {
            let version_inst =
                semver::Version::parse(installed_version).map_err(|error| ProtoError::Semver {
                    version: installed_version.to_owned(),
                    error,
                })?;

            if req.matches(&version_inst) {
                fully_qualified = true;
                maybe_version = installed_version.to_owned();

                return Ok(true);
            }
        }

        Ok(false)
    };

    // ^18 || ^20
    if version.contains("||") {
        for split_version in version.split("||") {
            if let Some(matched_version) = expand_detected_version(split_version.trim(), manifest)?
            {
                return Ok(Some(matched_version));
            }
        }

        // >=18, <20
    } else if version.contains(", ") {
        check_manifest(version.clone())?;

        // >=18 <20
    } else if version.contains(' ') {
        // Node.js doesn't require the comma, but Rust does
        check_manifest(version.replace(' ', ", "))?;

        // ^18, ~17, >16, ...
    } else {
        match &version[0..1] {
            "^" | "~" | ">" | "<" | "*" => {
                check_manifest(version.clone())?;
            }
            "=" => {
                maybe_version = version[1..].to_owned();
            }
            _ => {
                // Only use an exact match when fully qualified,
                // otherwise check the manifest against the partial.
                let dot_count = version.match_indices('.').collect::<Vec<_>>().len();

                if dot_count == 2 || !check_manifest(format!("^{version}"))? {
                    maybe_version = version.clone();
                }
            }
        };
    }

    if maybe_version.is_empty() {
        if version == "*" {
            return Ok(Some("latest".to_owned()));
        }

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
