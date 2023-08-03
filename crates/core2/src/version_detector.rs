use crate::error::ProtoError;
use crate::tool::Tool;
use crate::tool_manifest::ToolManifest;
use crate::tools_config::ToolsConfig;
use crate::version::{AliasOrVersion, VersionType};
use crate::version_resolver::resolve_version;
use std::{env, path::Path};
use tracing::{debug, trace};

pub async fn detect_version(
    tool: &Tool,
    forced_version: Option<AliasOrVersion>,
) -> miette::Result<AliasOrVersion> {
    let mut candidate = forced_version;

    // Env var takes highest priority
    if candidate.is_none() {
        let env_var = format!("{}_VERSION", tool.get_env_var_prefix());

        if let Ok(session_version) = env::var(&env_var) {
            debug!(
                tool = &tool.id,
                env_var,
                version = session_version,
                "Detected version from environment variable",
            );

            candidate = Some(AliasOrVersion::try_from(session_version)?);
        } else {
            trace!(
                tool = &tool.id,
                "Attempting to find local version from config files"
            );
        }
    } else {
        debug!(
            tool = &tool.id,
            version = ?candidate,
            "Using explicit version passed on the command line",
        );
    }

    // Traverse upwards and attempt to detect a local version
    if let Ok(working_dir) = env::current_dir() {
        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            // We already found a version, so exit
            if candidate.is_some() {
                break;
            }

            trace!(
                tool = &tool.id,
                dir = ?dir,
                "Checking directory",
            );

            // Detect from our config file
            let config = ToolsConfig::load_from(dir)?;

            if let Some(local_version) = config.tools.get(&tool.id) {
                debug!(
                    tool = &tool.id,
                    version = ?local_version,
                    file = ?config.path,
                    "Detected version from .prototools file",
                );

                candidate = Some(local_version.to_owned());
                break;
            }

            // Detect using the tool
            if let Some(detected_version) = tool.detect_version_from(dir).await? {
                if let Some(eco_version) =
                    expand_detected_version(&detected_version, &tool.manifest)?
                {
                    debug!(
                        tool = &tool.id,
                        version = ?eco_version,
                        "Detected version from tool's ecosystem"
                    );

                    candidate = Some(eco_version);
                    break;
                }
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
    if candidate.is_none() {
        trace!(
            tool = &tool.id,
            "Attempting to use global version from manifest",
        );

        if let Some(global_version) = &tool.manifest.default_version {
            debug!(
                tool = &tool.id,
                version = ?global_version,
                file = ?tool.manifest.path,
                "Detected global version from manifest",
            );

            candidate = Some(global_version.to_owned());
        }
    }

    // We didn't find anything!
    candidate.ok_or_else(|| {
        ProtoError::VersionDetectFailed {
            tool: tool.id.to_owned(),
        }
        .into()
    })
}

pub fn expand_detected_version(
    candidate: &VersionType,
    manifest: &ToolManifest,
) -> miette::Result<Option<AliasOrVersion>> {
    if let VersionType::Alias(alias) = candidate {
        return Ok(Some(AliasOrVersion::Alias(alias.to_owned())));
    }

    let versions = manifest.installed_versions.iter().collect::<Vec<_>>();

    if let Ok(version) = resolve_version(candidate, &versions, &manifest.aliases) {
        return Ok(Some(AliasOrVersion::Version(version)));
    }

    Ok(None)
}
