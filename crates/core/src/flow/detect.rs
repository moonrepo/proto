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

/// Detects versions from the environment.
pub struct Detector<'tool> {
    tool: &'tool Tool,

    pub detected_source: Option<PathBuf>,
}

impl<'tool> Detector<'tool> {
    pub fn new(tool: &'tool Tool) -> Self {
        Self {
            tool,
            detected_source: None,
        }
    }

    /// Detect a version using all available strategies.
    #[instrument(skip(self))]
    pub async fn detect_version(&mut self) -> Result<ToolSpec, ProtoDetectError> {
        // Env var takes highest priority
        let env_var = format!("{}_VERSION", self.tool.get_env_var_prefix());

        if let Ok(session_version) = env::var(&env_var)
            && !session_version.is_empty()
        {
            debug!(
                tool = self.tool.context.as_str(),
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
            tool = self.tool.context.as_str(),
            "Attempting to find version from {} files", PROTO_CONFIG_NAME
        );

        let config_files = self.tool.proto.load_config_files()?;
        let config = self.tool.proto.load_config()?;

        let detected_version = match config.settings.detect_strategy {
            DetectStrategy::FirstAvailable => {
                self.detect_version_first_available(&config_files).await?
            }
            DetectStrategy::PreferPrototools => {
                self.detect_version_prefer_prototools(&config_files).await?
            }
            DetectStrategy::OnlyPrototools => {
                self.detect_version_only_prototools(&config_files).await?
            }
        };

        if let Some(version) = detected_version {
            return Ok(version.into());
        }

        // We didn't find anything!
        Err(ProtoDetectError::FailedVersionDetect {
            tool: self.tool.get_name().to_owned(),
        })
    }

    /// Attempt to detect a version from the provided directory by scanning for applicable files.
    #[instrument(skip(self))]
    pub async fn detect_version_from(
        &mut self,
        current_dir: &Path,
    ) -> Result<Option<(UnresolvedVersionSpec, PathBuf)>, ProtoDetectError> {
        if !self
            .tool
            .plugin
            .has_func(PluginFunction::DetectVersionFiles)
            .await
        {
            return Ok(None);
        }

        let has_parser = self
            .tool
            .plugin
            .has_func(PluginFunction::ParseVersionFile)
            .await;
        let output: DetectVersionOutput = self
            .tool
            .plugin
            .cache_func_with(
                PluginFunction::DetectVersionFiles,
                DetectVersionInput {
                    context: self.tool.create_plugin_unresolved_context(),
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
            tool = self.tool.context.as_str(),
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
                    .tool
                    .plugin
                    .call_func_with(
                        PluginFunction::ParseVersionFile,
                        ParseVersionFileInput {
                            content,
                            context: self.tool.create_plugin_unresolved_context(),
                            file: file.clone(),
                            path: self.tool.to_virtual_path(&file_path),
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
                tool = self.tool.context.as_str(),
                file = ?file_path,
                version = version.to_string(),
                "Detected a version"
            );

            return Ok(Some((version, file_path)));
        }

        Ok(None)
    }

    pub async fn detect_version_first_available(
        &mut self,
        config_files: &[&ProtoConfigFile],
    ) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
        for file in config_files {
            if let Some(version) = self.detect_from_proto_config(file)? {
                return Ok(Some(version));
            }

            if let Some(version) = self.detect_from_tool_ecosystem(file).await? {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    pub async fn detect_version_only_prototools(
        &mut self,
        config_files: &[&ProtoConfigFile],
    ) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
        for file in config_files {
            if let Some(version) = self.detect_from_proto_config(file)? {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    #[instrument(skip_all)]
    pub async fn detect_version_prefer_prototools(
        &mut self,
        config_files: &[&ProtoConfigFile],
    ) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
        // Check config files first
        if let Some(version) = self.detect_version_only_prototools(config_files).await? {
            return Ok(Some(version));
        }

        // Then check the ecosystem
        for file in config_files {
            if let Some(version) = self.detect_from_tool_ecosystem(file).await? {
                return Ok(Some(version));
            }
        }

        Ok(None)
    }

    fn detect_from_proto_config(
        &mut self,
        file: &ProtoConfigFile,
    ) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
        if let Some(versions) = &file.config.versions
            && let Some(version) = versions.get(&self.tool.context)
        {
            debug!(
                tool = self.tool.context.as_str(),
                version = version.to_string(),
                file = ?file.path,
                "Detected version from {} file", PROTO_CONFIG_NAME
            );

            self.detected_source = Some(file.path.clone());

            return Ok(Some(version.req.to_owned()));
        }

        Ok(None)
    }

    async fn detect_from_tool_ecosystem(
        &mut self,
        file: &ProtoConfigFile,
    ) -> Result<Option<UnresolvedVersionSpec>, ProtoDetectError> {
        if let Some((version, file)) = self
            .detect_version_from(file.path.parent().unwrap())
            .await?
        {
            debug!(
                tool = self.tool.context.as_str(),
                version = version.to_string(),
                file = ?file,
                "Detected version from tool's ecosystem"
            );

            self.detected_source = Some(file);

            return Ok(Some(version));
        }

        Ok(None)
    }
}
