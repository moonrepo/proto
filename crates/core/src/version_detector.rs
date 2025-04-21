use crate::flow::resolve::ProtoResolveError;
use crate::tool::Tool;
use crate::{ToolSpec, config::*};
use std::env;
use std::path::Path;
use tracing::instrument;
use tracing::{debug, trace};
use version_spec::*;

fn set_detected_env_var(prefix: String, path: &Path) {
    unsafe { env::set_var(format!("{prefix}_DETECTED_FROM"), path) };
}

#[instrument(name = "first_available", skip_all)]
pub async fn detect_version_first_available(
    tool: &Tool,
    config_files: &[&ProtoConfigFile],
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    for file in config_files {
        if let Some(versions) = &file.config.versions {
            if let Some(version) = versions.get(tool.id.as_str()) {
                debug!(
                    tool = tool.id.as_str(),
                    version = version.to_string(),
                    file = ?file.path,
                    "Detected version from {} file", PROTO_CONFIG_NAME
                );

                set_detected_env_var(tool.get_env_var_prefix(), &file.path);

                return Ok(Some(version.req.to_owned()));
            }
        }

        let dir = file.path.parent().unwrap();

        if let Some((version, file)) = tool.detect_version_from(dir).await? {
            debug!(
                tool = tool.id.as_str(),
                version = version.to_string(),
                file = ?file,
                "Detected version from tool's ecosystem"
            );

            set_detected_env_var(tool.get_env_var_prefix(), &file);

            return Ok(Some(version));
        }
    }

    Ok(None)
}

#[instrument(name = "only_prototools", skip_all)]
pub async fn detect_version_only_prototools(
    tool: &Tool,
    config_files: &[&ProtoConfigFile],
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    for file in config_files {
        if let Some(versions) = &file.config.versions {
            if let Some(version) = versions.get(tool.id.as_str()) {
                debug!(
                    tool = tool.id.as_str(),
                    version = version.to_string(),
                    file = ?file.path,
                    "Detected version from {} file", PROTO_CONFIG_NAME
                );

                set_detected_env_var(tool.get_env_var_prefix(), &file.path);

                return Ok(Some(version.req.to_owned()));
            }
        }
    }

    Ok(None)
}

#[instrument(name = "prefer_prototools", skip_all)]
pub async fn detect_version_prefer_prototools(
    tool: &Tool,
    config_files: &[&ProtoConfigFile],
) -> miette::Result<Option<UnresolvedVersionSpec>> {
    // Check config files first
    if let Some(version) = detect_version_only_prototools(tool, config_files).await? {
        return Ok(Some(version));
    }

    // Then check the ecosystem
    for file in config_files {
        let dir = file.path.parent().unwrap();

        if let Some((version, file)) = tool.detect_version_from(dir).await? {
            debug!(
                tool = tool.id.as_str(),
                version = version.to_string(),
                file = ?file,
                "Detected version from tool's ecosystem"
            );

            set_detected_env_var(tool.get_env_var_prefix(), &file);

            return Ok(Some(version));
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn detect_version(tool: &Tool) -> miette::Result<ToolSpec> {
    // Env var takes highest priority
    let env_var = format!("{}_VERSION", tool.get_env_var_prefix());

    if let Ok(session_version) = env::var(&env_var) {
        if !session_version.is_empty() {
            debug!(
                tool = tool.id.as_str(),
                env_var,
                version = session_version,
                "Detected version from environment variable",
            );

            return Ok(UnresolvedVersionSpec::parse(&session_version)
                .map_err(|error| ProtoResolveError::InvalidVersionSpec {
                    version: session_version,
                    error: Box::new(error),
                })?
                .into());
        }
    }

    // Traverse upwards and attempt to detect a version
    trace!(
        tool = tool.id.as_str(),
        "Attempting to find version from {} files", PROTO_CONFIG_NAME
    );

    let config_files = tool.proto.load_config_files()?;
    let config = tool.proto.load_config()?;

    let detected_version = match config.settings.detect_strategy {
        DetectStrategy::FirstAvailable => {
            detect_version_first_available(tool, &config_files).await?
        }
        DetectStrategy::PreferPrototools => {
            detect_version_prefer_prototools(tool, &config_files).await?
        }
        DetectStrategy::OnlyPrototools => {
            detect_version_only_prototools(tool, &config_files).await?
        }
    };

    if let Some(version) = detected_version {
        return Ok(version.into());
    }

    // We didn't find anything!
    Err(ProtoResolveError::FailedVersionDetect {
        tool: tool.get_name().to_owned(),
    }
    .into())
}
