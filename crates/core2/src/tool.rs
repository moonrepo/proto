use crate::proto::ProtoEnvironment;
use crate::tool_manifest::ToolManifest;
use crate::version::DetectedVersion;
use extism::Manifest as PluginManifest;
use miette::IntoDiagnostic;
use proto_pdk_api::*;
use starbase_utils::fs;
use std::env::{self, consts};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use warpgate::PluginContainer;

pub struct Tool {
    pub id: String,
    pub manifest: ToolManifest,
    pub plugin: PluginContainer<'static>,
    pub proto: ProtoEnvironment,
}

// HELPERS

impl Tool {
    pub fn load(id: &str, proto: &ProtoEnvironment) -> miette::Result<Self> {
        let manifest = ToolManifest::load_from(proto.tools_dir.join(id))?;

        // TODO
        let plugin = PluginContainer::new_without_functions(id, PluginManifest::default())?;

        Ok(Tool {
            id: id.to_owned(),
            manifest,
            plugin,
            proto: proto.to_owned(),
        })
    }

    pub fn get_tool_dir(&self) -> PathBuf {
        self.proto.tools_dir.join(&self.id)
    }
}

// APIs

impl Tool {
    pub fn get_environment(&self) -> miette::Result<Environment> {
        Ok(Environment {
            arch: HostArch::from_str(consts::ARCH).into_diagnostic()?,
            id: self.id.clone(),
            os: HostOS::from_str(consts::OS).into_diagnostic()?,
            vars: self
                .get_metadata()?
                .env_vars
                .iter()
                .filter_map(|var| env::var(var).ok().map(|value| (var.to_owned(), value)))
                .collect(),
            // TODO
            version: String::new(), // self.get_resolved_version().to_owned(),
        })
    }

    pub fn get_metadata(&self) -> miette::Result<ToolMetadataOutput> {
        self.plugin.cache_func_with(
            "register_tool",
            ToolMetadataInput {
                id: self.id.clone(),
                env: Environment {
                    arch: HostArch::from_str(consts::ARCH).into_diagnostic()?,
                    id: self.id.clone(),
                    os: HostOS::from_str(consts::OS).into_diagnostic()?,
                    ..Environment::default()
                },
            },
        )
    }
}

// DETECTION

impl Tool {
    /// Attempt to detect an applicable version from the provided directory.
    pub async fn detect_version_from(
        &self,
        current_dir: &Path,
    ) -> miette::Result<Option<DetectedVersion>> {
        if !self.plugin.has_func("detect_version_files") {
            return Ok(None);
        }

        let has_parser = self.plugin.has_func("parse_version_file");
        let result: DetectVersionOutput = self.plugin.cache_func("detect_version_files")?;

        for file in result.files {
            let file_path = current_dir.join(&file);

            if !file_path.exists() {
                continue;
            }

            let content = fs::read_file(&file_path)?;

            let version = if has_parser {
                let result: ParseVersionFileOutput = self.plugin.call_func_with(
                    "parse_version_file",
                    ParseVersionFileInput {
                        content,
                        env: self.get_environment()?,
                        file: file.clone(),
                    },
                )?;

                if result.version.is_none() {
                    continue;
                }

                result.version.unwrap()
            } else {
                content
            };

            return Ok(Some(DetectedVersion::try_from(version)?));
        }

        Ok(None)
    }
}
