use crate::error::ProtoError;
use crate::helpers::{is_cache_enabled, is_offline, remove_v_prefix};
use crate::proto::ProtoEnvironment;
use crate::shimmer::{create_global_shim, create_local_shim, ShimContext};
use crate::tool_manifest::ToolManifest;
use crate::version::{AliasOrVersion, VersionType};
use crate::version_resolver::VersionResolver;
use extism::Manifest as PluginManifest;
use miette::IntoDiagnostic;
use proto_pdk_api::*;
use starbase_utils::{fs, json};
use std::env::{self, consts};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime};
use tracing::debug;
use warpgate::PluginContainer;

pub struct Tool {
    pub id: String,
    pub manifest: ToolManifest,
    pub plugin: PluginContainer<'static>,
    pub proto: ProtoEnvironment,

    version: Option<AliasOrVersion>,
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

    /// Return the prefix for environment variable names.
    pub fn get_env_var_prefix(&self) -> String {
        format!("PROTO_{}", self.id.to_uppercase().replace('-', "_"))
    }

    /// Return an absolute path to the tool's root directory that contains installed versions,
    /// the manifest, possible globals, and more.
    pub fn get_inventory_dir(&self) -> PathBuf {
        self.proto.tools_dir.join(&self.id)
    }

    /// Return a human readable name for the tool.
    pub fn get_name(&self) -> String {
        self.get_metadata().unwrap().name
    }

    /// Return the resolved version or "latest".
    pub fn get_resolved_version(&self) -> AliasOrVersion {
        self.version
            .clone()
            .unwrap_or_else(|| AliasOrVersion::Alias("latest".into()))
    }

    /// Return an absolute path to the tool's install directory for the currently resolved version.
    pub fn get_tool_dir(&self) -> PathBuf {
        self.get_inventory_dir()
            .join(self.get_resolved_version().to_string())
    }

    /// Explicitly set the version to use.
    pub fn set_version(&mut self, version: AliasOrVersion) {
        self.version = Some(version);
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
            version: self.get_resolved_version().to_string(),
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
    /// Load the available versions to install and return a resolver instance.
    /// To reduce network overhead, cache the results for 24 hours.
    pub async fn load_version_resolver(
        &self,
        initial_version: &str,
    ) -> miette::Result<VersionResolver> {
        debug!(tool = &self.id, "Loading available versions");

        let mut versions: Option<LoadVersionsOutput> = None;
        let cache_path = self
            .proto
            .temp_dir
            .join(format!("{}-versions.json", self.id));

        // Attempt to read from the cache first
        if cache_path.exists() && (is_cache_enabled() || is_offline()) {
            let metadata = fs::metadata(&cache_path)?;

            // If offline, always use the cache, otherwise only within the last 24 hours
            let read_cache = if is_offline() {
                true
            } else if let Ok(modified_time) = metadata.modified().or_else(|_| metadata.created()) {
                modified_time > SystemTime::now() - Duration::from_secs(60 * 60 * 24)
            } else {
                false
            };

            if read_cache {
                debug!(tool = &self.id, cache = ?cache_path, "Loading from local cache");

                versions = Some(json::read_file(&cache_path)?);
            }
        }

        // Nothing cached, so load from the plugin
        if versions.is_none() {
            if is_offline() {
                return Err(ProtoError::InternetConnectionRequired.into());
            }

            versions = Some(self.plugin.cache_func_with(
                "load_versions",
                LoadVersionsInput {
                    env: self.get_environment()?,
                    initial: initial_version.to_owned(),
                },
            )?);
        }

        // Cache the results and create a resolver
        let versions = versions.unwrap();
        json::write_file(cache_path, &versions, false)?;

        let mut resolver = VersionResolver::from_output(versions);
        resolver.inherit_aliases(&self.manifest);

        Ok(resolver)
    }

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// (or alias) according to the tool's ecosystem.
    pub async fn resolve_version(
        &mut self,
        initial_version: &str,
    ) -> miette::Result<AliasOrVersion> {
        if let Some(version) = &self.version {
            return Ok(version.to_owned());
        }

        let initial_version = remove_v_prefix(initial_version).to_lowercase();

        // If offline but we have a fully qualified semantic version,
        // exit early and assume the version is legitimate!
        if is_offline() {
            if let Ok(version) = Version::parse(&initial_version) {
                return Ok(AliasOrVersion::Version(version));
            }
        }

        debug!(
            tool = &self.id,
            initial_version = initial_version,
            "Resolving a semantic version",
        );

        let resolver = self.load_version_resolver(&initial_version).await?;
        let mut version = AliasOrVersion::Alias("latest".into());
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
                version = AliasOrVersion::Version(resolver.resolve(candidate)?);
            }

            if let Some(candidate) = result.version {
                debug!(
                    tool = &self.id,
                    version = &candidate,
                    "Received an explicit version or alias to use",
                );

                resolved = true;
                version = AliasOrVersion::try_from(candidate).into_diagnostic()?;
            }
        }

        if !resolved {
            version = AliasOrVersion::Version(resolver.resolve(initial_version)?);
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

// INSTALLATION

impl Tool {
    pub async fn setup(&mut self, initial_version: &str) -> miette::Result<bool> {
        Ok(true)
    }

    pub async fn teardown(&mut self) -> miette::Result<bool> {
        Ok(true)
    }
}

// SHIMMER

impl Tool {
    /// Create the context object required for creating shim files.
    pub fn create_shim_context(&self) -> ShimContext {
        let mut context = ShimContext {
            shim_file: &self.id,
            bin: &self.id,
            tool_id: &self.id,
            tool_dir: Some(self.get_tool_dir()),
            ..ShimContext::default()
        };

        if let AliasOrVersion::Version(version) = self.get_resolved_version() {
            context.tool_version = Some(version.to_string());
        }

        context
    }

    /// Create global and local shim files for the current tool.
    /// If find only is enabled, will only check if they exist, and not create.
    pub fn create_shims(&self, find_only: bool) -> miette::Result<()> {
        let mut primary_context = self.create_shim_context();

        // If not configured from the plugin, always create the primary global
        if !self.plugin.has_func("create_shims") {
            create_global_shim(&self.proto, primary_context, find_only)?;

            return Ok(());
        }

        let shim_configs: CreateShimsOutput = self.plugin.cache_func_with(
            "create_shims",
            CreateShimsInput {
                env: self.get_environment()?,
            },
        )?;

        // Create the primary global shim
        if let Some(primary_config) = &shim_configs.primary {
            primary_context.before_args = primary_config.before_args.as_deref();
            primary_context.after_args = primary_config.after_args.as_deref();
        }

        if !shim_configs.no_primary_global {
            create_global_shim(&self.proto, primary_context, find_only)?;
        }

        // Create alternate/secondary global shims
        for (name, config) in &shim_configs.global_shims {
            let mut context = self.create_shim_context();
            context.shim_file = name;
            context.bin_path = config.bin_path.as_deref();
            context.before_args = config.before_args.as_deref();
            context.after_args = config.after_args.as_deref();

            create_global_shim(&self.proto, context, find_only)?;
        }

        // Create local shims
        for (name, config) in &shim_configs.local_shims {
            let bin_path = if let Some(path) = &config.bin_path {
                self.get_tool_dir().join(path)
            } else {
                self.get_tool_dir().join(&self.id)
            };

            let mut context = self.create_shim_context();
            context.shim_file = name;
            context.bin_path = Some(&bin_path);
            context.parent_bin = config.parent_bin.as_deref();
            context.before_args = config.before_args.as_deref();
            context.after_args = config.after_args.as_deref();

            create_local_shim(context, find_only)?;
        }

        Ok(())
    }
}
