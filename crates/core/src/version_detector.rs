use crate::error::ProtoError;
use crate::proto_config::*;
use crate::tool::Tool;
use std::env;
use tracing::{debug, trace};
use version_spec::*;

pub async fn detect_version_first_available(
    tool: &Tool,
    config_manager: &ProtoConfigManager,
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    for file in &config_manager.files {
        if let Some(versions) = &file.config.versions {
            if let Some(version) = versions.get(tool.id.as_str()) {
                debug!(
                    tool = tool.id.as_str(),
                    version = version.to_string(),
                    file = ?file.path,
                    "Detected version from {} file", PROTO_CONFIG_NAME
                );

                return Ok(Some(version.to_owned()));
            }
        }

        let dir = file.path.parent().unwrap();

        if let Some(version) = tool.detect_version_from(dir).await? {
            debug!(
                tool = tool.id.as_str(),
                version = version.to_string(),
                dir = ?dir,
                "Detected version from tool's ecosystem"
            );

            return Ok(Some(version));
        }
    }

    Ok(None)
}

pub async fn detect_version_prefer_prototools(
    tool: &Tool,
    config_manager: &ProtoConfigManager,
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    // Check config files first
    for file in &config_manager.files {
        if let Some(versions) = &file.config.versions {
            if let Some(version) = versions.get(tool.id.as_str()) {
                debug!(
                    tool = tool.id.as_str(),
                    version = version.to_string(),
                    file = ?file.path,
                    "Detected version from {} file", PROTO_CONFIG_NAME
                );

                return Ok(Some(version.to_owned()));
            }
        }
    }

    // Then check the ecosystem
    for file in &config_manager.files {
        let dir = file.path.parent().unwrap();

        if let Some(version) = tool.detect_version_from(dir).await? {
            debug!(
                tool = tool.id.as_str(),
                version = version.to_string(),
                dir = ?dir,
                "Detected version from tool's ecosystem"
            );

            return Ok(Some(version));
        }
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
    }

    // Traverse upwards and attempt to detect a version
    trace!(
        tool = tool.id.as_str(),
        "Attempting to find version from {} files",
        PROTO_CONFIG_NAME
    );

    let config_manager = tool.proto.load_config_manager()?;
    let config = tool.proto.load_config()?;

    let detected_version = match config.settings.detect_strategy {
        DetectStrategy::FirstAvailable => {
            detect_version_first_available(tool, config_manager).await?
        }
        DetectStrategy::PreferPrototools => {
            detect_version_prefer_prototools(tool, config_manager).await?
        }
    };

    if let Some(version) = detected_version {
        return Ok(version);
    }

    // We didn't find anything!
    Err(ProtoError::VersionDetectFailed {
        tool: tool.get_name().to_owned(),
    }
    .into())
}
