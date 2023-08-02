use crate::manifest::Manifest;
use crate::proto::ProtoEnvironment;
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
    pub plugin: PluginContainer<'static>,

    manifest: Manifest,
    proto: ProtoEnvironment,
}

// HELPERS

impl Tool {
    pub fn load(id: &str, proto: &ProtoEnvironment) -> miette::Result<Self> {
        let manifest = Manifest::load_from(proto.tools_dir.join(id))?;

        // TODO
        let plugin = PluginContainer::new_without_functions(id, PluginManifest::default())?;

        Ok(Tool {
            id: id.to_owned(),
            manifest,
            plugin,
            proto: proto.to_owned(),
        })
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn get_manifest_mut(&mut self) -> &mut Manifest {
        &mut self.manifest
    }

    pub fn get_tool_dir(&self) -> PathBuf {
        self.proto.tools_dir.join(self.get_id())
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
    /// Attempt to detect an applicable version from the provided working directory.
    pub async fn detect_version_from(&self, working_dir: &Path) -> miette::Result<Option<String>> {
        if !self.plugin.has_func("detect_version_files") {
            return Ok(None);
        }

        let has_parser = self.plugin.has_func("parse_version_file");
        let result: DetectVersionOutput = self.plugin.cache_func("detect_version_files")?;

        for file in result.files {
            let file_path = working_dir.join(&file);

            if !file_path.exists() {
                continue;
            }

            if has_parser {
                let result: ParseVersionFileOutput = self.plugin.call_func_with(
                    "parse_version_file",
                    ParseVersionFileInput {
                        content: fs::read_file(&file_path)?,
                        env: self.get_environment()?,
                        file: file.clone(),
                    },
                )?;

                if result.version.is_none() {
                    continue;
                }

                return Ok(result.version);
            }

            // TODO
            // return Ok(Some(load_version_file(&file_path)?));
        }

        Ok(None)
    }
}
