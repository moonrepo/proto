pub use super::detect_error::ProtoDetectError;
use crate::config::{DetectStrategy, PROTO_CONFIG_NAME};
use crate::file_manager::ProtoConfigFile;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::*;
use starbase_utils::fs;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, trace};

fn set_detected_env_var(prefix: String, path: &Path) {
    unsafe { env::set_var(format!("{prefix}_DETECTED_FROM"), path) };
}

fn detect_from_proto_config(
    tool: &Tool,
    file: &ProtoConfigFile,
) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
    if let Some(versions) = &file.config.versions
        && let Some(version) = versions.get(&tool.context)
    {
        debug!(
            tool = tool.context.as_str(),
            version = version.to_string(),
            file = ?file.path,
            "Detected version from {} file", PROTO_CONFIG_NAME
        );

        set_detected_env_var(tool.get_env_var_prefix(), &file.path);

        return Ok(Some(version.req.to_owned()));
    }

    Ok(None)
}

async fn detect_from_tool_ecosystem(
    tool: &Tool,
    file: &ProtoConfigFile,
) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
    if let Some((version, file)) = tool
        .detect_version_from(file.path.parent().unwrap())
        .await?
    {
        debug!(
            tool = tool.context.as_str(),
            version = version.to_string(),
            file = ?file,
            "Detected version from tool's ecosystem"
        );

        set_detected_env_var(tool.get_env_var_prefix(), &file);

        return Ok(Some(version));
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn detect_version_first_available(
    tool: &Tool,
    config_files: &[&ProtoConfigFile],
) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
    for file in config_files {
        if let Some(version) = detect_from_proto_config(tool, file)? {
            return Ok(Some(version));
        }

        if let Some(version) = detect_from_tool_ecosystem(tool, file).await? {
            return Ok(Some(version));
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn detect_version_only_prototools(
    tool: &Tool,
    config_files: &[&ProtoConfigFile],
) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
    for file in config_files {
        if let Some(version) = detect_from_proto_config(tool, file)? {
            return Ok(Some(version));
        }
    }

    Ok(None)
}

#[instrument(skip_all)]
pub async fn detect_version_prefer_prototools(
    tool: &Tool,
    config_files: &[&ProtoConfigFile],
) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
    // Check config files first
    if let Some(version) = detect_version_only_prototools(tool, config_files).await? {
        return Ok(Some(version));
    }

    // Then check the ecosystem
    for file in config_files {
        if let Some(version) = detect_from_tool_ecosystem(tool, file).await? {
            return Ok(Some(version));
        }
    }

    Ok(None)
}

impl Tool {
    #[instrument(skip(self))]
    pub async fn detect_version(&self) -> Result<ToolSpec, ProtoDetectError> {
        // Env var takes highest priority
        let env_var = format!("{}_VERSION", self.get_env_var_prefix());

        if let Ok(session_version) = env::var(&env_var)
            && !session_version.is_empty()
        {
            debug!(
                tool = self.context.as_str(),
                env_var,
                version = session_version,
                "Detected version from environment variable",
            );

            return Ok(UnresolvedVersionSpec::parse(&session_version)
                .map_err(|error| ProtoDetectError::InvalidDetectedVersionSpec {
                    path: PathBuf::from(env_var),
                    version: session_version,
                    error: Box::new(error),
                })?
                .into());
        }

        // Traverse upwards and attempt to detect a version
        trace!(
            tool = self.context.as_str(),
            "Attempting to find version from {} files", PROTO_CONFIG_NAME
        );

        let config_files = self.proto.load_config_files()?;
        let config = self.proto.load_config()?;

        let detected_version = match config.settings.detect_strategy {
            DetectStrategy::FirstAvailable => {
                detect_version_first_available(self, &config_files).await?
            }
            DetectStrategy::PreferPrototools => {
                detect_version_prefer_prototools(self, &config_files).await?
            }
            DetectStrategy::OnlyPrototools => {
                detect_version_only_prototools(self, &config_files).await?
            }
        };

        if let Some(version) = detected_version {
            return Ok(version.into());
        }

        // We didn't find anything!
        Err(ProtoDetectError::FailedVersionDetect {
            tool: self.get_name().to_owned(),
        })
    }

    /// Attempt to detect a version from the provided directory by scanning for applicable files.
    #[instrument(skip(self))]
    pub async fn detect_version_from(
        &self,
        current_dir: &Path,
    ) -> Result<Option<(UnresolvedVersionSpec, PathBuf)>, ProtoDetectError> {
        if !self
            .plugin
            .has_func(PluginFunction::DetectVersionFiles)
            .await
        {
            return Ok(None);
        }

        let has_parser = self.plugin.has_func(PluginFunction::ParseVersionFile).await;
        let output: DetectVersionOutput = self
            .plugin
            .cache_func_with(
                PluginFunction::DetectVersionFiles,
                DetectVersionInput {
                    context: self.create_plugin_unresolved_context(),
                },
            )
            .await?;

        if !output.ignore.is_empty()
            && let Some(dir) = current_dir.to_str()
            && output.ignore.iter().any(|ignore| dir.contains(ignore))
        {
            return Ok(None);
        }

        trace!(
            tool = self.context.as_str(),
            dir = ?current_dir,
            "Attempting to detect a version from directory"
        );

        for file in output.files {
            let file_path = current_dir.join(&file);

            if !file_path.exists() {
                continue;
            }

            let content = fs::read_file(&file_path)?.trim().to_owned();

            if content.is_empty() {
                continue;
            }

            let version = if has_parser {
                let output: ParseVersionFileOutput = self
                    .plugin
                    .call_func_with(
                        PluginFunction::ParseVersionFile,
                        ParseVersionFileInput {
                            content,
                            context: self.create_plugin_unresolved_context(),
                            file: file.clone(),
                            path: self.to_virtual_path(&file_path),
                        },
                    )
                    .await?;

                if output.version.is_none() {
                    continue;
                }

                output.version.unwrap()
            } else {
                UnresolvedVersionSpec::parse(&content).map_err(|error| {
                    ProtoDetectError::InvalidDetectedVersionSpec {
                        error: Box::new(error),
                        path: file_path.clone(),
                        version: content,
                    }
                })?
            };

            debug!(
                tool = self.context.as_str(),
                file = ?file_path,
                version = version.to_string(),
                "Detected a version"
            );

            return Ok(Some((version, file_path)));
        }

        Ok(None)
    }
}
