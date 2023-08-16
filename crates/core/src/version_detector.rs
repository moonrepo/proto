use crate::error::ProtoError;
use crate::tool::Tool;
use crate::tools_config::ToolsConfig;
use crate::version::VersionType;
use std::{env, path::Path};
use tracing::{debug, trace};

pub async fn detect_version(
    tool: &Tool,
    forced_version: Option<VersionType>,
) -> miette::Result<VersionType> {
    let mut candidate = forced_version;

    // Env var takes highest priority
    if candidate.is_none() {
        let env_var = format!("{}_VERSION", tool.get_env_var_prefix());

        if let Ok(session_version) = env::var(&env_var) {
            debug!(
                tool = tool.id.as_str(),
                env_var,
                version = session_version,
                "Detected version from environment variable",
            );

            candidate = Some(VersionType::parse(session_version)?);
        } else {
            trace!(
                tool = tool.id.as_str(),
                "Attempting to find local version from config files"
            );
        }
    } else {
        debug!(
            tool = tool.id.as_str(),
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
                tool = tool.id.as_str(),
                dir = ?dir,
                "Checking directory",
            );

            // Detect from our config file
            let config = ToolsConfig::load_from(dir)?;

            if let Some(local_version) = config.tools.get(&tool.id) {
                debug!(
                    tool = tool.id.as_str(),
                    version = ?local_version,
                    file = ?config.path,
                    "Detected version from .prototools file",
                );

                candidate = Some(local_version.to_implicit_type());
                break;
            }

            // Detect using the tool
            if let Some(detected_version) = tool.detect_version_from(dir).await? {
                debug!(
                    tool = tool.id.as_str(),
                    version = detected_version.to_string(),
                    "Detected version from tool's ecosystem"
                );

                candidate = Some(detected_version);
                break;
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
    if candidate.is_none() {
        trace!(
            tool = tool.id.as_str(),
            "Attempting to use global version from manifest",
        );

        if let Some(global_version) = &tool.manifest.default_version {
            debug!(
                tool = tool.id.as_str(),
                version = global_version.to_string(),
                file = ?tool.manifest.path,
                "Detected global version from manifest",
            );

            candidate = Some(global_version.to_implicit_type());
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
