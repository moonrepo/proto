use crate::error::ProtoError;
use crate::tool::Tool;
use crate::tools_config::ToolsConfig;
use std::{env, path::Path};
use tracing::{debug, trace};
use version_spec::*;

pub async fn detect_version(
    tool: &Tool,
    forced_version: Option<UnresolvedVersionSpec>,
) -> miette::Result<UnresolvedVersionSpec> {
    if let Some(candidate) = forced_version {
        debug!(
            tool = tool.id.as_str(),
            version = ?candidate,
            "Using explicit version passed on the command line",
        );

        return Ok(candidate);
    }

    // Env var takes highest priorit
    let env_var = format!("{}_VERSION", tool.get_env_var_prefix());

    if let Ok(session_version) = env::var(&env_var) {
        debug!(
            tool = tool.id.as_str(),
            env_var,
            version = session_version,
            "Detected version from environment variable",
        );

        return Ok(
            UnresolvedVersionSpec::parse(&session_version).map_err(|error| ProtoError::Semver {
                version: session_version,
                error,
            })?,
        );
    } else {
        trace!(
            tool = tool.id.as_str(),
            "Attempting to find local version from config files"
        );
    }

    // Traverse upwards and attempt to detect a local version
    if let Ok(working_dir) = env::current_dir() {
        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = current_dir {
            // Don't traverse past the home directory
            if dir == tool.proto.home {
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

                return Ok(local_version.to_owned());
            }

            // Detect using the tool
            if let Some(detected_version) = tool.detect_version_from(dir).await? {
                debug!(
                    tool = tool.id.as_str(),
                    version = detected_version.to_string(),
                    "Detected version from tool's ecosystem"
                );

                return Ok(detected_version);
            }

            current_dir = dir.parent();
        }
    }

    // If still no version, load the global version
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

        return Ok(global_version.to_owned());
    }

    // We didn't find anything!
    Err(ProtoError::VersionDetectFailed {
        tool: tool.id.to_owned(),
    }
    .into())
}
