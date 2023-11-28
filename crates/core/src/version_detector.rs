use crate::error::ProtoError;
use crate::tool::Tool;
use crate::tools_config::ToolsConfig;
use crate::user_config::DetectStrategy;
use std::{env, path::Path};
use tracing::{debug, trace};
use version_spec::*;

pub async fn detect_version_first_available(
    tool: &Tool,
    start_dir: &Path,
    end_dir: &Path,
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    let mut current_dir: Option<&Path> = Some(start_dir);

    while let Some(dir) = current_dir {
        trace!(
            tool = tool.id.as_str(),
            dir = ?dir,
            "Checking directory",
        );

        let config = ToolsConfig::load_from(dir)?;

        if let Some(version) = config.tools.get(&tool.id) {
            debug!(
                tool = tool.id.as_str(),
                version = version.to_string(),
                file = ?config.path,
                "Detected version from .prototools file",
            );

            return Ok(Some(version.to_owned()));
        }

        if let Some(version) = tool.detect_version_from(dir).await? {
            debug!(
                tool = tool.id.as_str(),
                version = version.to_string(),
                "Detected version from tool's ecosystem"
            );

            return Ok(Some(version));
        }

        if dir == end_dir {
            break;
        }

        current_dir = dir.parent();
    }

    Ok(None)
}

pub async fn detect_version_prefer_prototools(
    tool: &Tool,
    start_dir: &Path,
    end_dir: &Path,
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    let mut config_version = None;
    let mut config_path = None;
    let mut ecosystem_version = None;
    let mut current_dir: Option<&Path> = Some(start_dir);

    while let Some(dir) = current_dir {
        trace!(
            tool = tool.id.as_str(),
            dir = ?dir,
            "Checking directory",
        );

        if config_version.is_none() {
            let mut config = ToolsConfig::load_from(dir)?;

            config_version = config.tools.remove(&tool.id);
            config_path = Some(config.path);
        }

        if ecosystem_version.is_none() {
            ecosystem_version = tool.detect_version_from(dir).await?;
        }

        if dir == end_dir {
            break;
        }

        current_dir = dir.parent();
    }

    if let Some(version) = config_version {
        debug!(
            tool = tool.id.as_str(),
            version = version.to_string(),
            file = ?config_path.unwrap(),
            "Detected version from .prototools file",
        );

        return Ok(Some(version.to_owned()));
    }

    if let Some(version) = ecosystem_version {
        debug!(
            tool = tool.id.as_str(),
            version = version.to_string(),
            "Detected version from tool's ecosystem"
        );

        return Ok(Some(version));
    }

    Ok(None)
}

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

    // Env var takes highest priority
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
        let user_config = tool.proto.load_user_config()?;
        let detected_version = match user_config.detect_strategy {
            DetectStrategy::FirstAvailable => {
                detect_version_first_available(tool, &working_dir, &tool.proto.home).await?
            }
            DetectStrategy::PreferPrototools => {
                detect_version_prefer_prototools(tool, &working_dir, &tool.proto.home).await?
            }
        };

        if let Some(version) = detected_version {
            return Ok(version);
        }
    }

    // If still no version, load the global version
    trace!(
        tool = tool.id.as_str(),
        "Attempting to use global version from user config",
    );

    let user_config = tool.proto.load_user_config()?;

    if let Some(tool_user_config) = user_config.tools.get(&tool.id) {
        if let Some(global_version) = &tool_user_config.default_version {
            debug!(
                tool = tool.id.as_str(),
                version = global_version.to_string(),
                file = ?user_config.path,
                "Detected global version from user config",
            );

            return Ok(global_version.to_owned());
        }
    }

    // We didn't find anything!
    Err(ProtoError::VersionDetectFailed {
        tool: tool.get_name().to_owned(),
    }
    .into())
}
