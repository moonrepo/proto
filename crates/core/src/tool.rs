use crate::checksum::verify_checksum;
use crate::error::ProtoError;
use crate::events::*;
use crate::helpers::{
    extract_filename_from_url, get_proto_version, is_archive_file, is_offline, ENV_VAR,
};
use crate::layout::{Inventory, Product};
use crate::proto::ProtoEnvironment;
use crate::proto_config::ProtoConfig;
use crate::shim_registry::{Shim, ShimRegistry, ShimsMap};
use crate::tool_manifest::ToolManifestVersion;
use crate::version_resolver::VersionResolver;
use miette::IntoDiagnostic;
use proto_pdk_api::*;
use proto_shim::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use starbase_archive::Archiver;
use starbase_events::Emitter;
use starbase_styles::color;
use starbase_utils::{fs, net};
use std::collections::BTreeMap;
use std::env;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, trace, warn};
use warpgate::{
    host_funcs::{create_host_functions, HostData},
    Id, PluginContainer, PluginLocator, PluginManifest, VirtualPath, Wasm,
};

#[derive(Debug, Default, Serialize)]
pub struct ExecutableLocation {
    pub config: ExecutableConfig,
    pub name: String,
    pub path: PathBuf,
    pub primary: bool,
}

pub struct Tool {
    pub id: Id,
    pub metadata: ToolMetadataOutput,
    pub locator: Option<PluginLocator>,
    pub plugin: Arc<PluginContainer>,
    pub proto: Arc<ProtoEnvironment>,
    pub version: Option<VersionSpec>,

    // Store
    pub inventory: Inventory,
    pub product: Product,

    // Events
    pub on_created_bins: Emitter<CreatedBinariesEvent>,
    pub on_created_shims: Emitter<CreatedShimsEvent>,
    pub on_installing: Emitter<InstallingEvent>,
    pub on_installed: Emitter<InstalledEvent>,
    pub on_resolved_version: Emitter<ResolvedVersionEvent>,
    pub on_uninstalling: Emitter<UninstallingEvent>,
    pub on_uninstalled: Emitter<UninstalledEvent>,

    cache: bool,
    exe_path: Option<PathBuf>,
    globals_dir: Option<PathBuf>,
    globals_prefix: Option<String>,
}

impl Tool {
    pub fn new(
        id: Id,
        proto: Arc<ProtoEnvironment>,
        plugin: Arc<PluginContainer>,
    ) -> miette::Result<Self> {
        debug!(
            "Created tool {} and its WASM runtime",
            color::id(id.as_str())
        );

        let mut tool = Tool {
            cache: true,
            exe_path: None,
            globals_dir: None,
            globals_prefix: None,
            id,
            inventory: Inventory::default(),
            locator: None,
            metadata: ToolMetadataOutput::default(),
            plugin,
            product: Product::default(),
            proto,
            version: None,

            // Events
            on_created_bins: Emitter::new(),
            on_created_shims: Emitter::new(),
            on_installing: Emitter::new(),
            on_installed: Emitter::new(),
            on_resolved_version: Emitter::new(),
            on_uninstalling: Emitter::new(),
            on_uninstalled: Emitter::new(),
        };

        tool.register_tool()?;

        Ok(tool)
    }

    pub fn load<I: AsRef<Id>, P: AsRef<ProtoEnvironment>>(
        id: I,
        proto: P,
        wasm: Wasm,
    ) -> miette::Result<Self> {
        let proto = proto.as_ref();

        Self::load_from_manifest(id, proto, Self::create_plugin_manifest(proto, wasm)?)
    }

    pub fn load_from_manifest<I: AsRef<Id>, P: AsRef<ProtoEnvironment>>(
        id: I,
        proto: P,
        manifest: PluginManifest,
    ) -> miette::Result<Self> {
        let id = id.as_ref();
        let proto = proto.as_ref();

        debug!(
            "Creating tool {} and instantiating plugin",
            color::id(id.as_str())
        );

        Self::new(
            id.to_owned(),
            Arc::new(proto.to_owned()),
            Arc::new(PluginContainer::new(
                id.to_owned(),
                manifest,
                create_host_functions(HostData {
                    virtual_paths: proto.get_virtual_paths(),
                    working_dir: proto.cwd.clone(),
                }),
            )?),
        )
    }

    pub fn create_plugin_manifest<P: AsRef<ProtoEnvironment>>(
        proto: P,
        wasm: Wasm,
    ) -> miette::Result<PluginManifest> {
        let proto = proto.as_ref();

        let mut manifest = PluginManifest::new([wasm]);
        manifest = manifest.with_allowed_host("*");
        manifest = manifest.with_allowed_paths(proto.get_virtual_paths().into_iter());
        manifest = manifest.with_timeout(Duration::from_secs(90));

        #[cfg(debug_assertions)]
        {
            manifest = manifest.with_timeout(Duration::from_secs(120));
        }

        Ok(manifest)
    }

    fn call_locate_executables(&self) -> miette::Result<LocateExecutablesOutput> {
        self.plugin.cache_func_with(
            "locate_executables",
            LocateExecutablesInput {
                context: self.create_context(),
            },
        )
    }

    /// Disable internal caching when applicable.
    pub fn disable_caching(&mut self) {
        self.cache = false;
    }

    /// Return the prefix for environment variable names.
    pub fn get_env_var_prefix(&self) -> String {
        format!("PROTO_{}", self.id.to_uppercase().replace('-', "_"))
    }

    /// Return an absolute path to the tool's inventory directory.
    /// The inventory houses installed versions, the manifest, and more.
    pub fn get_inventory_dir(&self) -> PathBuf {
        self.inventory.dir.clone()
    }

    /// Return a human readable name for the tool.
    pub fn get_name(&self) -> &str {
        &self.metadata.name
    }

    /// Return the resolved version or "latest".
    pub fn get_resolved_version(&self) -> VersionSpec {
        self.version.clone().unwrap_or_default()
    }

    /// Return an absolute path to a temp directory solely for this tool.
    pub fn get_temp_dir(&self) -> PathBuf {
        self.inventory
            .temp_dir
            .join(self.get_resolved_version().to_string())
    }

    /// Return an absolute path to the tool's install directory for the currently resolved version.
    pub fn get_product_dir(&self) -> PathBuf {
        self.product.dir.clone()
    }

    /// Explicitly set the version to use.
    pub fn set_version(&mut self, version: VersionSpec) {
        self.product = self.inventory.create_product(&version);
        self.version = Some(version);
    }

    /// Disable progress bars when installing or uninstalling the tool.
    pub fn disable_progress_bars(&self) -> bool {
        self.metadata.inventory.disable_progress_bars
    }

    /// Convert a virtual path to a real path.
    pub fn from_virtual_path(&self, path: &Path) -> PathBuf {
        self.plugin.from_virtual_path(path)
    }

    /// Convert a real path to a virtual path.
    pub fn to_virtual_path(&self, path: &Path) -> VirtualPath {
        self.plugin.to_virtual_path(path)
    }
}

// APIs

impl Tool {
    /// Return contextual information to pass to WASM plugin functions.
    pub fn create_context(&self) -> ToolContext {
        ToolContext {
            proto_version: Some(get_proto_version()),
            tool_dir: self.to_virtual_path(&self.get_product_dir()),
            version: self.get_resolved_version(),
        }
    }

    /// Register the tool by loading initial metadata and persisting it.
    pub fn register_tool(&mut self) -> miette::Result<()> {
        let metadata: ToolMetadataOutput = self.plugin.cache_func_with(
            "register_tool",
            ToolMetadataInput {
                id: self.id.to_string(),
            },
        )?;

        let mut inventory = self
            .proto
            .store
            .create_inventory(&self.id, &metadata.inventory)?;

        if let Some(override_dir) = &metadata.inventory.override_dir {
            let override_dir_path = override_dir.real_path();

            if override_dir_path.is_none() || override_dir_path.is_some_and(|p| p.is_relative()) {
                return Err(ProtoError::AbsoluteInventoryDir {
                    tool: self.get_name().to_owned(),
                }
                .into());
            }

            inventory.dir = self.from_virtual_path(override_dir);
        }

        self.product = inventory.create_product(&self.get_resolved_version());
        self.inventory = inventory;
        self.metadata = metadata;

        Ok(())
    }

    /// Sync the local tool manifest with changes from the plugin.
    pub fn sync_manifest(&mut self) -> miette::Result<()> {
        if !self.plugin.has_func("sync_manifest") {
            return Ok(());
        }

        debug!(tool = self.id.as_str(), "Syncing manifest with changes");

        let sync_changes: SyncManifestOutput = self.plugin.call_func_with(
            "sync_manifest",
            SyncManifestInput {
                context: self.create_context(),
            },
        )?;

        if sync_changes.skip_sync {
            return Ok(());
        }

        let mut modified = false;
        let manifest = &mut self.inventory.manifest;

        if let Some(versions) = sync_changes.versions {
            modified = true;

            let mut entries = FxHashMap::default();
            let mut installed = FxHashSet::default();

            for version in versions {
                let key = VersionSpec::Version(version);
                let value = manifest.versions.get(&key).cloned().unwrap_or_default();

                installed.insert(key.clone());
                entries.insert(key, value);
            }

            manifest.versions = entries;
            manifest.installed_versions = installed;
        }

        if modified {
            manifest.save()?;
        }

        Ok(())
    }
}

// VERSION RESOLUTION

impl Tool {
    /// Load available versions to install and return a resolver instance.
    /// To reduce network overhead, results will be cached for 24 hours.
    pub async fn load_version_resolver(
        &self,
        initial_version: &UnresolvedVersionSpec,
    ) -> miette::Result<VersionResolver> {
        debug!(tool = self.id.as_str(), "Loading available versions");

        let mut versions = LoadVersionsOutput::default();
        let mut cached = false;

        if let Some(cached_versions) = self.inventory.load_remote_versions(!self.cache)? {
            versions = cached_versions;
            cached = true;
        }

        // Nothing cached, so load from the plugin
        if !cached {
            if is_offline() {
                return Err(ProtoError::InternetConnectionRequiredForVersion {
                    command: format!("{}_VERSION=1.2.3 {}", self.get_env_var_prefix(), self.id),
                    bin_dir: self.proto.store.bin_dir.clone(),
                }
                .into());
            }

            if env::var("PROTO_BYPASS_VERSION_CHECK").is_err() {
                versions = self.plugin.cache_func_with(
                    "load_versions",
                    LoadVersionsInput {
                        initial: initial_version.to_owned(),
                    },
                )?;

                self.inventory.save_remote_versions(&versions)?;
            }
        }

        // Cache the results and create a resolver
        let mut resolver = VersionResolver::from_output(versions);

        resolver.with_manifest(&self.inventory.manifest);

        let config = self.proto.load_config()?;

        if let Some(tool_config) = config.tools.get(&self.id) {
            resolver.with_config(tool_config);
        }

        Ok(resolver)
    }

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// (or alias) according to the tool's ecosystem.
    pub async fn resolve_version(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
        short_circuit: bool,
    ) -> miette::Result<()> {
        if self.version.is_some() {
            return Ok(());
        }

        debug!(
            tool = self.id.as_str(),
            initial_version = initial_version.to_string(),
            "Resolving a semantic version or alias",
        );

        // If we have a fully qualified semantic version,
        // exit early and assume the version is legitimate!
        // Also canary is a special type that we can simply just use.
        if short_circuit && matches!(initial_version, UnresolvedVersionSpec::Version(_))
            || matches!(initial_version, UnresolvedVersionSpec::Canary)
        {
            let version = initial_version.to_resolved_spec();

            debug!(
                tool = self.id.as_str(),
                version = version.to_string(),
                "Resolved to {} (without validation)",
                version
            );

            self.on_resolved_version
                .emit(ResolvedVersionEvent {
                    candidate: initial_version.to_owned(),
                    version: version.clone(),
                })
                .await?;

            self.set_version(version);

            return Ok(());
        }

        let resolver = self.load_version_resolver(initial_version).await?;
        let handle_error = || ProtoError::VersionResolveFailed {
            tool: self.get_name().to_owned(),
            version: initial_version.to_string(),
        };

        let mut version = VersionSpec::default();
        let mut resolved = false;

        if self.plugin.has_func("resolve_version") {
            let result: ResolveVersionOutput = self.plugin.call_func_with(
                "resolve_version",
                ResolveVersionInput {
                    initial: initial_version.to_owned(),
                },
            )?;

            if let Some(candidate) = result.candidate {
                debug!(
                    tool = self.id.as_str(),
                    candidate = candidate.to_string(),
                    "Received a possible version or alias to use",
                );

                resolved = true;
                version = resolver.resolve(&candidate).ok_or_else(handle_error)?;
            }

            if let Some(candidate) = result.version {
                debug!(
                    tool = self.id.as_str(),
                    version = candidate.to_string(),
                    "Received an explicit version or alias to use",
                );

                resolved = true;
                version = candidate;
            }
        }

        if !resolved {
            version = resolver.resolve(initial_version).ok_or_else(handle_error)?;
        }

        debug!(
            tool = self.id.as_str(),
            version = version.to_string(),
            "Resolved to {}",
            version
        );

        self.on_resolved_version
            .emit(ResolvedVersionEvent {
                candidate: initial_version.to_owned(),
                version: version.clone(),
            })
            .await?;

        self.set_version(version);

        Ok(())
    }

    /// Attempt to detect an applicable version from the provided directory.
    pub async fn detect_version_from(
        &self,
        current_dir: &Path,
    ) -> miette::Result<Option<(UnresolvedVersionSpec, PathBuf)>> {
        if !self.plugin.has_func("detect_version_files") {
            return Ok(None);
        }

        let has_parser = self.plugin.has_func("parse_version_file");
        let result: DetectVersionOutput = self.plugin.cache_func("detect_version_files")?;

        if !result.ignore.is_empty() {
            if let Some(dir) = current_dir.to_str() {
                if result.ignore.iter().any(|ignore| dir.contains(ignore)) {
                    return Ok(None);
                }
            }
        }

        trace!(
            tool = self.id.as_str(),
            dir = ?current_dir,
            "Attempting to detect a version from directory"
        );

        for file in result.files {
            let file_path = current_dir.join(&file);

            if !file_path.exists() {
                continue;
            }

            let content = fs::read_file(&file_path)?.trim().to_owned();

            if content.is_empty() {
                continue;
            }

            let version = if has_parser {
                let result: ParseVersionFileOutput = self.plugin.call_func_with(
                    "parse_version_file",
                    ParseVersionFileInput {
                        content,
                        file: file.clone(),
                    },
                )?;

                if result.version.is_none() {
                    continue;
                }

                result.version.unwrap()
            } else {
                UnresolvedVersionSpec::parse(&content).map_err(|error| ProtoError::Semver {
                    version: content,
                    error,
                })?
            };

            debug!(
                tool = self.id.as_str(),
                file = ?file_path,
                version = version.to_string(),
                "Detected a version"
            );

            return Ok(Some((version, file_path)));
        }

        Ok(None)
    }
}

// INSTALLATION

impl Tool {
    /// Return true if the tool has been installed. This is less accurate than `is_setup`,
    /// as it only checks for the existence of the inventory directory.
    pub fn is_installed(&self) -> bool {
        let dir = self.get_product_dir();

        self.version
            .as_ref()
            // Canary can be overwritten so treat as not-installed
            .is_some_and(|v| {
                !v.is_latest()
                    && !v.is_canary()
                    && self.inventory.manifest.installed_versions.contains(v)
            })
            && dir.exists()
            && !fs::is_dir_locked(dir)
    }

    /// Verify the downloaded file using the checksum strategy for the tool.
    /// Common strategies are SHA256 and MD5.
    pub async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
        checksum_public_key: Option<&str>,
    ) -> miette::Result<bool> {
        debug!(
            tool = self.id.as_str(),
            download_file = ?download_file,
            checksum_file = ?checksum_file,
            "Verifiying checksum of downloaded file",
        );

        // Allow plugin to provide their own checksum verification method
        let verified = if self.plugin.has_func("verify_checksum") {
            let result: VerifyChecksumOutput = self.plugin.call_func_with(
                "verify_checksum",
                VerifyChecksumInput {
                    checksum_file: self.to_virtual_path(checksum_file),
                    download_file: self.to_virtual_path(download_file),
                    context: self.create_context(),
                },
            )?;

            result.verified

        // Otherwise attempt to verify it ourselves
        } else {
            verify_checksum(download_file, checksum_file, checksum_public_key)?
        };

        if verified {
            debug!(
                tool = self.id.as_str(),
                "Successfully verified, checksum matches"
            );

            return Ok(true);
        }

        Err(ProtoError::InvalidChecksum {
            checksum: checksum_file.to_path_buf(),
            download: download_file.to_path_buf(),
        }
        .into())
    }

    pub async fn build_from_source(&self, _install_dir: &Path) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool by building from source"
        );

        if !self.plugin.has_func("build_instructions") {
            return Err(ProtoError::UnsupportedBuildFromSource {
                tool: self.get_name().to_owned(),
            }
            .into());
        }

        // let temp_dir = self.get_temp_dir();

        // let options: BuildInstructionsOutput = self.plugin.cache_func_with(
        //     "build_instructions",
        //     BuildInstructionsInput {
        //         context: self.create_context(),
        //     },
        // )?;

        // match &options.source {
        //     // Should this do anything?
        //     SourceLocation::None => {
        //         return Ok(());
        //     }

        //     // Download from archive
        //     SourceLocation::Archive { url: archive_url } => {
        //         let download_file = temp_dir.join(extract_filename_from_url(archive_url)?);

        //         debug!(
        //             tool = self.id.as_str(),
        //             archive_url,
        //             download_file = ?download_file,
        //             install_dir = ?install_dir,
        //             "Attempting to download and unpack sources",
        //         );

        //         net::download_from_url_with_client(
        //             archive_url,
        //             &download_file,
        //             self.proto.get_plugin_loader()?.get_client()?,
        //         )
        //         .await?;

        //         Archiver::new(install_dir, &download_file).unpack_from_ext()?;
        //     }

        //     // Clone from Git repository
        //     SourceLocation::Git {
        //         url: repo_url,
        //         reference: ref_name,
        //         submodules,
        //     } => {
        //         debug!(
        //             tool = self.id.as_str(),
        //             repo_url,
        //             ref_name,
        //             install_dir = ?install_dir,
        //             "Attempting to clone a Git repository",
        //         );

        //         let run_git = |args: &[&str]| -> miette::Result<()> {
        //             let status = Command::new("git")
        //                 .args(args)
        //                 .current_dir(install_dir)
        //                 .spawn()
        //                 .into_diagnostic()?
        //                 .wait()
        //                 .into_diagnostic()?;

        //             if !status.success() {
        //                 return Err(ProtoError::BuildFailed {
        //                     tool: self.get_name().to_owned(),
        //                     url: repo_url.clone(),
        //                     status: format!("exit code {}", status),
        //                 }
        //                 .into());
        //             }

        //             Ok(())
        //         };

        //         // TODO, pull if already cloned

        //         fs::create_dir_all(install_dir)?;

        //         run_git(&[
        //             "clone",
        //             if *submodules {
        //                 "--recurse-submodules"
        //             } else {
        //                 ""
        //             },
        //             repo_url,
        //             ".",
        //         ])?;

        //         run_git(&["checkout", ref_name])?;
        //     }
        // };

        Ok(())
    }

    /// Download the tool (as an archive) from its distribution registry
    /// into the `~/.proto/tools/<version>` folder, and optionally verify checksums.
    pub async fn install_from_prebuilt(&self, install_dir: &Path) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool from a pre-built archive"
        );

        let client = self.proto.get_plugin_loader()?.get_client()?;
        let options: DownloadPrebuiltOutput = self.plugin.cache_func_with(
            "download_prebuilt",
            DownloadPrebuiltInput {
                context: self.create_context(),
                install_dir: self.to_virtual_path(install_dir),
            },
        )?;

        let temp_dir = self.get_temp_dir();

        // Download the prebuilt
        let download_url = options.download_url;
        let download_file = match options.download_name {
            Some(name) => temp_dir.join(name),
            None => temp_dir.join(extract_filename_from_url(&download_url)?),
        };

        if download_file.exists() {
            debug!(
                tool = self.id.as_str(),
                "Tool already downloaded, continuing"
            );
        } else {
            debug!(tool = self.id.as_str(), "Tool not downloaded, downloading");

            net::download_from_url_with_client(&download_url, &download_file, client).await?;
        }

        // Verify the checksum if applicable
        if let Some(checksum_url) = options.checksum_url {
            let checksum_file = temp_dir.join(match options.checksum_name {
                Some(name) => name,
                None => extract_filename_from_url(&checksum_url)?,
            });

            if !checksum_file.exists() {
                debug!(
                    tool = self.id.as_str(),
                    "Checksum does not exist, downloading"
                );

                net::download_from_url_with_client(&checksum_url, &checksum_file, client).await?;
            }

            self.verify_checksum(
                &checksum_file,
                &download_file,
                options.checksum_public_key.as_deref(),
            )
            .await?;
        }

        // Attempt to unpack the archive
        debug!(
            tool = self.id.as_str(),
            download_file = ?download_file,
            install_dir = ?install_dir,
            "Attempting to unpack archive",
        );

        if self.plugin.has_func("unpack_archive") {
            self.plugin.call_func_without_output(
                "unpack_archive",
                UnpackArchiveInput {
                    input_file: self.to_virtual_path(&download_file),
                    output_dir: self.to_virtual_path(install_dir),
                    context: self.create_context(),
                },
            )?;
        }
        // Is an archive, unpack it
        else if is_archive_file(&download_file) {
            let mut archiver = Archiver::new(install_dir, &download_file);

            if let Some(prefix) = &options.archive_prefix {
                archiver.set_prefix(prefix);
            }

            let (ext, unpacked_path) = archiver.unpack_from_ext()?;

            // If the archive was `.gz` without tar or other formats,
            // it's a single file, so assume a binary and update perms
            if ext == "gz" && unpacked_path.is_file() {
                fs::update_perms(unpacked_path, None)?;
            }
        }
        // Not an archive, assume a binary and copy
        else {
            let install_path = install_dir.join(get_exe_file_name(&self.id));

            fs::rename(&download_file, &install_path)?;
            fs::update_perms(install_path, None)?;
        }

        Ok(())
    }

    /// Install a tool into proto, either by downloading and unpacking
    /// a pre-built archive, or by using a native installation method.
    pub async fn install(&mut self, _build: bool) -> miette::Result<bool> {
        if self.is_installed() {
            debug!(
                tool = self.id.as_str(),
                "Tool already installed, continuing"
            );

            return Ok(false);
        }

        if is_offline() {
            return Err(ProtoError::InternetConnectionRequired.into());
        }

        let install_dir = self.get_product_dir();
        let mut installed = false;

        // Lock the install directory. If the inventory has been overridden,
        // lock the internal proto tool directory instead.
        let install_lock = fs::lock_directory(if self.metadata.inventory.override_dir.is_some() {
            self.proto
                .store
                .inventory_dir
                .join(self.id.as_str())
                .join(self.get_resolved_version().to_string())
        } else {
            install_dir.clone()
        })?;

        self.on_installing
            .emit(InstallingEvent {
                version: self.get_resolved_version(),
            })
            .await?;

        // If this function is defined, it acts like an escape hatch and
        // takes precedence over all other install strategies
        if self.plugin.has_func("native_install") {
            debug!(tool = self.id.as_str(), "Installing tool natively");

            let result: NativeInstallOutput = self.plugin.call_func_with(
                "native_install",
                NativeInstallInput {
                    context: self.create_context(),
                    install_dir: self.to_virtual_path(&install_dir),
                },
            )?;

            if !result.installed && !result.skip_install {
                return Err(ProtoError::InstallFailed {
                    tool: self.get_name().to_owned(),
                    error: result.error.unwrap_or_default(),
                }
                .into());

            // If native install fails, attempt other installers
            } else {
                installed = result.installed;
            }
        }

        if !installed {
            // // Build the tool from source
            // if build {
            //     self.build_from_source(&install_dir).await?;

            // // Install from a prebuilt archive
            // } else {
            //     self.install_from_prebuilt(&install_dir).await?;
            // }

            self.install_from_prebuilt(&install_dir).await?;
        }

        install_lock.unlock()?;

        self.on_installed
            .emit(InstalledEvent {
                version: self.get_resolved_version(),
            })
            .await?;

        debug!(
            tool = self.id.as_str(),
            install_dir = ?install_dir,
            "Successfully installed tool",
        );

        Ok(true)
    }

    /// Uninstall the tool by deleting the current install directory.
    pub async fn uninstall(&self) -> miette::Result<bool> {
        let install_dir = self.get_product_dir();

        if !install_dir.exists() {
            debug!(
                tool = self.id.as_str(),
                "Tool has not been installed, aborting"
            );

            return Ok(false);
        }

        self.on_uninstalling
            .emit(UninstallingEvent {
                version: self.get_resolved_version(),
            })
            .await?;

        if self.plugin.has_func("native_uninstall") {
            debug!(tool = self.id.as_str(), "Uninstalling tool natively");

            let result: NativeUninstallOutput = self.plugin.call_func_with(
                "native_uninstall",
                NativeUninstallInput {
                    context: self.create_context(),
                },
            )?;

            if !result.uninstalled && !result.skip_uninstall {
                return Err(ProtoError::UninstallFailed {
                    tool: self.get_name().to_owned(),
                    error: result.error.unwrap_or_default(),
                }
                .into());
            }
        }

        debug!(
            tool = self.id.as_str(),
            install_dir = ?install_dir,
            "Deleting install directory"
        );

        fs::remove_dir_all(install_dir)?;

        self.on_uninstalled
            .emit(UninstalledEvent {
                version: self.get_resolved_version(),
            })
            .await?;

        debug!(tool = self.id.as_str(), "Successfully uninstalled tool");

        Ok(true)
    }
}

// BINARIES, SHIMS

impl Tool {
    /// Create all executables for the current tool.
    /// - Locate the primary binary to execute.
    /// - Generate shims to `~/.proto/shims`.
    /// - Symlink bins to `~/.proto/bin`.
    pub async fn create_executables(
        &mut self,
        force_shims: bool,
        force_bins: bool,
    ) -> miette::Result<()> {
        self.locate_executable().await?;
        self.generate_shims(force_shims).await?;
        self.symlink_bins(force_bins).await?;

        Ok(())
    }

    /// Return an absolute path to the executable file for the tool.
    pub fn get_exe_path(&self) -> miette::Result<&Path> {
        self.exe_path.as_deref().ok_or_else(|| {
            ProtoError::UnknownTool {
                id: self.id.clone(),
            }
            .into()
        })
    }

    /// Return an absolute path to the globals directory in which packages are installed to.
    pub fn get_globals_bin_dir(&self) -> Option<&Path> {
        self.globals_dir.as_deref()
    }

    /// Return a string that all globals are prefixed with. Will be used for filtering and listing.
    pub fn get_globals_prefix(&self) -> Option<&str> {
        self.globals_prefix.as_deref()
    }

    /// Return a list of all binaries that get created in `~/.proto/bin`.
    /// The list will contain the executable config, and an absolute path
    /// to the binaries final location.
    pub fn get_bin_locations(&self) -> miette::Result<Vec<ExecutableLocation>> {
        let options = self.call_locate_executables()?;
        let mut locations = vec![];

        let mut add = |name: &str, config: ExecutableConfig, primary: bool| {
            if !config.no_bin
                && config
                    .exe_link_path
                    .as_ref()
                    .or(config.exe_path.as_ref())
                    .is_some()
            {
                locations.push(ExecutableLocation {
                    path: self.proto.store.bin_dir.join(get_exe_file_name(name)),
                    name: name.to_owned(),
                    config,
                    primary,
                });
            }
        };

        if let Some(primary) = options.primary {
            add(&self.id, primary, true);
        }

        for (name, secondary) in options.secondary {
            add(&name, secondary, false);
        }

        Ok(locations)
    }

    /// Return location information for the primary executable within the tool directory.
    pub fn get_exe_location(&self) -> miette::Result<Option<ExecutableLocation>> {
        let options = self.call_locate_executables()?;

        if let Some(primary) = options.primary {
            if let Some(exe_path) = &primary.exe_path {
                return Ok(Some(ExecutableLocation {
                    path: self.get_product_dir().join(exe_path),
                    name: self.id.to_string(),
                    config: primary,
                    primary: true,
                }));
            }
        }

        Ok(None)
    }

    /// Return a list of all shims that get created in `~/.proto/shims`.
    /// The list will contain the executable config, and an absolute path
    /// to the shims final location.
    pub fn get_shim_locations(&self) -> miette::Result<Vec<ExecutableLocation>> {
        let options = self.call_locate_executables()?;
        let mut locations = vec![];

        let mut add = |name: &str, config: ExecutableConfig, primary: bool| {
            if !config.no_shim {
                locations.push(ExecutableLocation {
                    path: self.proto.store.shims_dir.join(get_shim_file_name(name)),
                    name: name.to_owned(),
                    config: config.clone(),
                    primary,
                });
            }
        };

        if let Some(primary) = options.primary {
            add(&self.id, primary, true);
        }

        for (name, secondary) in options.secondary {
            add(&name, secondary, false);
        }

        Ok(locations)
    }

    /// Locate the primary executable from the tool directory.
    pub async fn locate_executable(&mut self) -> miette::Result<()> {
        debug!(tool = self.id.as_str(), "Locating executable for tool");

        let exe_path = if let Some(location) = self.get_exe_location()? {
            location.path
        } else {
            self.get_product_dir().join(self.id.as_str())
        };

        if exe_path.exists() {
            debug!(tool = self.id.as_str(), exe_path = ?exe_path, "Found an executable");

            self.exe_path = Some(exe_path);

            return Ok(());
        }

        Err(ProtoError::MissingToolExecutable {
            tool: self.get_name().to_owned(),
            path: exe_path,
        }
        .into())
    }

    /// Locate the directory that global packages are installed to.
    pub async fn locate_globals_dir(&mut self) -> miette::Result<()> {
        if !self.plugin.has_func("locate_executables") || self.globals_dir.is_some() {
            return Ok(());
        }

        debug!(
            tool = self.id.as_str(),
            "Locating globals bin directory for tool"
        );

        let install_dir = self.get_product_dir();
        let options = self.call_locate_executables()?;

        self.globals_prefix = options.globals_prefix;

        // Find a globals directory that packages are installed to
        let lookup_count = options.globals_lookup_dirs.len() - 1;

        'outer: for (index, dir_lookup) in options.globals_lookup_dirs.iter().enumerate() {
            let mut dir = dir_lookup.clone();

            // If a lookup contains an env var, find and replace it.
            // If the var is not defined or is empty, skip this lookup.
            for cap in ENV_VAR.captures_iter(dir_lookup) {
                let var = cap.get(0).unwrap().as_str();

                let var_value = match var {
                    "$HOME" => self.proto.home.to_string_lossy().to_string(),
                    "$PROTO_HOME" => self.proto.root.to_string_lossy().to_string(),
                    "$PROTO_ROOT" => self.proto.root.to_string_lossy().to_string(),
                    "$TOOL_DIR" => install_dir.to_string_lossy().to_string(),
                    _ => env::var(cap.get(1).unwrap().as_str()).unwrap_or_default(),
                };

                if var_value.is_empty() {
                    continue 'outer;
                }

                dir = dir.replace(var, &var_value);
            }

            let dir_path = if let Some(dir_suffix) = dir.strip_prefix('~') {
                self.proto.home.join(dir_suffix)
            } else {
                PathBuf::from(dir)
            };

            if dir_path.exists() || index == lookup_count {
                debug!(tool = self.id.as_str(), bin_dir = ?dir_path, "Found a globals directory");

                self.globals_dir = Some(dir_path);
                break;
            }
        }

        Ok(())
    }

    /// Create shim files for the current tool if they are missing or out of date.
    /// If find only is enabled, will only check if they exist, and not create.
    pub async fn generate_shims(&mut self, force: bool) -> miette::Result<()> {
        let shims = self.get_shim_locations()?;

        if shims.is_empty() {
            return Ok(());
        }

        let is_outdated = self.inventory.manifest.shim_version != SHIM_VERSION;
        let force_create = force || is_outdated;
        let find_only = !force_create;

        if force_create {
            debug!(
                tool = self.id.as_str(),
                shims_dir = ?self.proto.store.shims_dir,
                shim_version = SHIM_VERSION,
                "Creating shims as they either do not exist, or are outdated"
            );

            self.inventory.manifest.shim_version = SHIM_VERSION;
            self.inventory.manifest.save()?;
        }

        let mut event = CreatedShimsEvent {
            global: vec![],
            local: vec![],
        };

        let mut registry: ShimsMap = BTreeMap::default();
        registry.insert(self.id.to_string(), Shim::default());

        fs::create_dir_all(&self.proto.store.shims_dir)?;

        for location in shims {
            let mut shim_entry = Shim::default();

            // Handle before and after args
            if let Some(before_args) = location.config.shim_before_args {
                shim_entry.before_args = match before_args {
                    StringOrVec::String(value) => shell_words::split(&value).into_diagnostic()?,
                    StringOrVec::Vec(value) => value,
                };
            }

            if let Some(after_args) = location.config.shim_after_args {
                shim_entry.after_args = match after_args {
                    StringOrVec::String(value) => shell_words::split(&value).into_diagnostic()?,
                    StringOrVec::Vec(value) => value,
                };
            }

            if let Some(env_vars) = location.config.shim_env_vars {
                shim_entry.env_vars.extend(env_vars);
            }

            if !location.primary {
                shim_entry.parent = Some(self.id.to_string());

                // Only use --alt when the secondary executable exists
                if location.config.exe_path.is_some() {
                    shim_entry.alt_bin = Some(true);
                }
            }

            // Create the shim file by copying the source bin
            self.proto.store.create_shim(&location.path, find_only)?;

            // Update the registry
            registry.insert(location.name.clone(), shim_entry);

            // Add to the event
            event.global.push(location.name);

            debug!(
                tool = self.id.as_str(),
                shim = ?location.path,
                shim_version = SHIM_VERSION,
                "Creating shim"
            );
        }

        self.on_created_shims.emit(event).await?;

        ShimRegistry::update(&self.proto, registry)?;

        Ok(())
    }

    /// Symlink all primary and secondary binaries for the current tool.
    pub async fn symlink_bins(&mut self, force: bool) -> miette::Result<()> {
        let bins = self.get_bin_locations()?;

        if bins.is_empty() {
            return Ok(());
        }

        if force {
            debug!(
                tool = self.id.as_str(),
                bins_dir = ?self.proto.store.bin_dir,
                "Creating symlinks to the original tool executables"
            );
        }

        fs::create_dir_all(&self.proto.store.bin_dir)?;

        let tool_dir = self.get_product_dir();
        let mut event = CreatedBinariesEvent { bins: vec![] };

        for location in bins {
            let input_path = tool_dir.join(
                location
                    .config
                    .exe_link_path
                    .as_ref()
                    .or(location.config.exe_path.as_ref())
                    .unwrap(),
            );
            let output_path = location.path;

            if !input_path.exists() {
                warn!(
                    tool = self.id.as_str(),
                    source = ?input_path,
                    target = ?output_path,
                    "Unable to symlink binary, source file does not exist"
                );

                continue;
            }

            if output_path.exists() && !force {
                continue;
            }

            debug!(
                tool = self.id.as_str(),
                source = ?input_path,
                target = ?output_path,
                "Creating binary symlink"
            );

            self.proto.store.unlink_bin(&output_path)?;
            self.proto.store.link_bin(&output_path, &input_path)?;

            event.bins.push(location.name);
        }

        self.on_created_bins.emit(event).await?;

        Ok(())
    }
}

// OPERATIONS

impl Tool {
    /// Return true if the tool has been setup (installed and binaries are located).
    pub async fn is_setup(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
    ) -> miette::Result<bool> {
        self.resolve_version(initial_version, true).await?;

        let install_dir = self.get_product_dir();

        debug!(
            tool = self.id.as_str(),
            install_dir = ?install_dir,
            "Checking if tool is installed",
        );

        if self.is_installed() {
            debug!(
                tool = self.id.as_str(),
                install_dir = ?install_dir,
                "Tool has already been installed, locating binaries and shims",
            );

            if self.exe_path.is_none() {
                self.create_executables(false, false).await?;
            }

            return Ok(true);
        } else {
            debug!(tool = self.id.as_str(), "Tool has not been installed");
        }

        Ok(false)
    }

    /// Setup the tool by resolving a semantic version, installing the tool,
    /// locating binaries, creating shims, and more.
    pub async fn setup(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
        build_from_source: bool,
    ) -> miette::Result<bool> {
        self.resolve_version(initial_version, false).await?;

        if !self.install(build_from_source).await? {
            return Ok(false);
        }

        self.create_executables(true, false).await?;
        self.cleanup().await?;

        let version = self.get_resolved_version();
        let default_version = self
            .metadata
            .default_version
            .clone()
            .unwrap_or_else(|| version.to_unresolved_spec());

        // Add version to manifest
        let manifest = &mut self.inventory.manifest;
        manifest.installed_versions.insert(version.clone());
        manifest
            .versions
            .insert(version.clone(), ToolManifestVersion::default());
        manifest.save()?;

        // Pin the global version
        ProtoConfig::update(self.proto.get_config_dir(true), |config| {
            config
                .versions
                .get_or_insert(Default::default())
                .entry(self.id.clone())
                .or_insert(default_version);
        })?;

        // Allow plugins to override manifest
        self.sync_manifest()?;

        Ok(true)
    }

    /// Teardown the tool by uninstalling the current version, removing the version
    /// from the manifest, and cleaning up temporary files. Return true if the teardown occurred.
    pub async fn teardown(&mut self) -> miette::Result<bool> {
        self.cleanup().await?;

        if !self.uninstall().await? {
            return Ok(false);
        }

        let version = self.get_resolved_version();
        let mut removed_default_version = false;

        // Remove version from manifest
        let manifest = &mut self.inventory.manifest;
        manifest.installed_versions.remove(&version);
        manifest.versions.remove(&version);
        manifest.save()?;

        // Unpin global version if a match
        ProtoConfig::update(self.proto.get_config_dir(true), |config| {
            if let Some(versions) = &mut config.versions {
                if versions.get(&self.id).is_some_and(|v| v == &version) {
                    debug!("Unpinning global version");

                    versions.remove(&self.id);
                    removed_default_version = true;
                }
            }
        })?;

        // If no more default version, delete the symlink,
        // otherwise the OS will throw errors for missing sources
        if removed_default_version || self.inventory.manifest.installed_versions.is_empty() {
            for bin in self.get_bin_locations()? {
                self.proto.store.unlink_bin(&bin.path)?;
            }
        }

        // If no more versions in general, delete all shims
        if self.inventory.manifest.installed_versions.is_empty() {
            for shim in self.get_shim_locations()? {
                self.proto.store.remove_shim(&shim.path)?;
            }
        }

        Ok(true)
    }

    /// Delete temporary files and downloads for the current version.
    pub async fn cleanup(&mut self) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Cleaning up temporary files and downloads"
        );

        fs::remove_dir_all(self.get_temp_dir())?;

        Ok(())
    }
}
