use crate::helpers::{is_offline, remove_v_prefix};
use crate::proto::ProtoEnvironment;
use crate::tool_manifest::ToolManifest;
use crate::version::{AliasOrVersion, VersionType};
use crate::version_resolver::VersionRegistry;
use crate::ProtoError;
use extism::Manifest as PluginManifest;
use miette::IntoDiagnostic;
use proto_pdk_api::*;
use starbase_utils::fs;
use std::env::{self, consts};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::debug;
use warpgate::PluginContainer;

pub struct Tool {
    pub id: String,
    pub manifest: ToolManifest,
    pub plugin: PluginContainer<'static>,
    pub proto: ProtoEnvironment,

    version: Option<Version>,
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
            version: None,
        })
    }

    /// Return an absolute path to the tool's directory that contains version installations.
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

// VERSION RESOLUTION

impl Tool {
    /// Load the available versions to install and return a registry.
    pub async fn load_version_registry(
        &self,
        initial_version: &str,
    ) -> miette::Result<VersionRegistry> {
        debug!(tool = &self.id, "Loading available versions");

        let mut available: LoadVersionsOutput = self.plugin.cache_func_with(
            "load_versions",
            LoadVersionsInput {
                env: self.get_environment()?,
                initial: initial_version.to_owned(),
            },
        )?;

        // Sort from newest to oldest
        available.versions.sort_by(|a, d| d.cmp(a));
        available.canary_versions.sort_by(|a, d| d.cmp(a));

        let mut registry = VersionRegistry::default();
        registry.versions.extend(available.versions);
        registry.aliases.extend(self.manifest.aliases.clone());

        for (alias, version) in available.aliases {
            registry
                .aliases
                .insert(alias, AliasOrVersion::Version(version));
        }

        if let Some(latest) = available.latest {
            registry
                .aliases
                .insert("latest".into(), AliasOrVersion::Version(latest));
        }

        Ok(registry)
    }

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// according to the tool's ecosystem.
    pub async fn resolve_version(&mut self, initial_version: &str) -> miette::Result<Version> {
        if let Some(version) = &self.version {
            return Ok(version.to_owned());
        }

        let initial_version = remove_v_prefix(initial_version).to_lowercase();

        // If offline but we have a fully qualified semantic version,
        // exit early and assume the version is legitimate!
        if is_offline() {
            if let Ok(version) = Version::parse(&initial_version) {
                return Ok(version);
            }
        }

        debug!(
            tool = &self.id,
            initial_version = initial_version,
            "Resolving a semantic version",
        );

        let registry = self.load_version_registry(&initial_version).await?;
        let mut version = Version::new(0, 0, 0);
        let mut resolved = false;

        if self.plugin.has_func("resolve_version") {
            let result: ResolveVersionOutput = self.plugin.call_func_with(
                "resolve_version",
                ResolveVersionInput {
                    env: self.get_environment()?,
                    initial: initial_version.to_owned(),
                },
            )?;

            if let Some(candidate) = result.candidate {
                debug!(
                    tool = &self.id,
                    candidate = &candidate,
                    "Received a possible version or alias to use",
                );

                resolved = true;
                version = registry.resolve(candidate)?;
            }

            if let Some(candidate) = result.version {
                debug!(
                    tool = &self.id,
                    version = &candidate,
                    "Received an explicit version to use",
                );

                resolved = true;
                version = Version::parse(&candidate).map_err(|error| ProtoError::Semver {
                    version: candidate,
                    error,
                })?;
            }
        }

        if !resolved {
            version = registry.resolve(initial_version)?;
        }

        debug!(
            tool = &self.id,
            version = version.to_string(),
            "Resolved to {}",
            version
        );

        self.version = Some(version.clone());

        Ok(version)
    }
}

// VERSION DETECTION

impl Tool {
    /// Attempt to detect an applicable version from the provided directory.
    pub async fn detect_version_from(
        &self,
        current_dir: &Path,
    ) -> miette::Result<Option<VersionType>> {
        if !self.plugin.has_func("detect_version_files") {
            return Ok(None);
        }

        let has_parser = self.plugin.has_func("parse_version_file");
        let result: DetectVersionOutput = self.plugin.cache_func("detect_version_files")?;

        debug!(
            tool = &self.id,
            dir = ?current_dir,
            files = ?result.files,
            "Attempting to detect a version from directory"
        );

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

            debug!(
                tool = &self.id,
                file = ?file_path,
                "Detected a version"
            );

            return Ok(Some(VersionType::try_from(version)?));
        }

        Ok(None)
    }
}
