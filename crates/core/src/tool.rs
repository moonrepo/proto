use crate::error::ProtoError;
use crate::events::*;
use crate::helpers::{
    extract_filename_from_url, hash_file_contents, is_archive_file, is_cache_enabled, is_offline,
    read_json_file_with_lock, write_json_file_with_lock, ENV_VAR,
};
use crate::proto::ProtoEnvironment;
use crate::shimmer::{
    create_global_shim, create_local_shim, get_shim_file_name, ShimContext, SHIM_VERSION,
};
use crate::tool_manifest::{now, ToolManifest};
use crate::version_resolver::VersionResolver;
use extism::{manifest::Wasm, Manifest as PluginManifest};
use miette::IntoDiagnostic;
use proto_pdk_api::*;
use proto_wasm_plugin::{create_host_functions, HostData};
use serde::Serialize;
use starbase_archive::Archiver;
use starbase_events::Emitter;
use starbase_styles::color;
use starbase_utils::fs;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fmt::Debug;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tracing::{debug, trace};
use version_spec::*;
use warpgate::{download_from_url_to_file, Id, PluginContainer, PluginLocator, VirtualPath};

pub struct Tool {
    pub id: Id,
    pub manifest: ToolManifest,
    pub metadata: ToolMetadataOutput,
    pub locator: Option<PluginLocator>,
    pub plugin: PluginContainer<'static>,
    pub proto: ProtoEnvironment,
    pub version: Option<VersionSpec>,

    // Events
    pub on_created_shims: Emitter<CreatedShimsEvent>,
    pub on_installing: Emitter<InstallingEvent>,
    pub on_installed: Emitter<InstalledEvent>,
    pub on_installed_global: Emitter<InstalledGlobalEvent>,
    pub on_resolved_version: Emitter<ResolvedVersionEvent>,
    pub on_uninstalling: Emitter<UninstallingEvent>,
    pub on_uninstalled: Emitter<UninstalledEvent>,
    pub on_uninstalled_global: Emitter<UninstalledGlobalEvent>,

    cache: bool,
    bin_path: Option<PathBuf>,
    globals_dir: Option<PathBuf>,
    globals_prefix: Option<String>,
}

impl Tool {
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

        let host_data = HostData {
            working_dir: proto.cwd.clone(),
        };

        let mut tool = Tool {
            bin_path: None,
            cache: true,
            globals_dir: None,
            globals_prefix: None,
            id: id.to_owned(),
            locator: None,
            manifest: ToolManifest::load_from(proto.tools_dir.join(id.as_str()))?,
            metadata: ToolMetadataOutput::default(),
            plugin: PluginContainer::new(
                id.to_owned(),
                manifest,
                create_host_functions(host_data),
            )?,
            proto: proto.to_owned(),
            version: None,

            // Events
            on_created_shims: Emitter::new(),
            on_installing: Emitter::new(),
            on_installed: Emitter::new(),
            on_installed_global: Emitter::new(),
            on_resolved_version: Emitter::new(),
            on_uninstalling: Emitter::new(),
            on_uninstalled: Emitter::new(),
            on_uninstalled_global: Emitter::new(),
        };

        if let Ok(level) = env::var("PROTO_WASM_LOG") {
            extism::set_log_file(
                proto.cwd.join("wasm-debug.log"),
                std::str::FromStr::from_str(&level).ok(),
            );
        }

        debug!(
            "Created tool {} and its WASM runtime",
            color::id(id.as_str())
        );

        tool.register_tool()?;

        Ok(tool)
    }

    pub fn create_plugin_manifest<P: AsRef<ProtoEnvironment>>(
        proto: P,
        wasm: Wasm,
    ) -> miette::Result<PluginManifest> {
        let proto = proto.as_ref();

        let mut manifest = PluginManifest::new([wasm]);
        manifest = manifest.with_allowed_host("*");

        #[cfg(debug_assertions)]
        {
            manifest = manifest.with_timeout(Duration::from_secs(90));
        }

        manifest = manifest.with_allowed_path(proto.cwd.clone(), "/workspace");
        manifest = manifest.with_allowed_path(proto.home.clone(), "/home");
        manifest = manifest.with_allowed_path(proto.root.clone(), "/proto");

        Ok(manifest)
    }

    /// Disable internal caching when applicable.
    pub fn disable_caching(&mut self) {
        self.cache = false;
    }

    /// Return the name of the executable binary, using proto's tool ID.
    pub fn get_bin_name(&self) -> String {
        if cfg!(windows) {
            format!("{}.exe", self.id)
        } else {
            self.id.to_string()
        }
    }

    /// Return an absolute path to the executable binary for the tool.
    pub fn get_bin_path(&self) -> miette::Result<&Path> {
        self.bin_path.as_deref().ok_or_else(|| {
            ProtoError::UnknownTool {
                id: self.id.clone(),
            }
            .into()
        })
    }

    /// Return the prefix for environment variable names.
    pub fn get_env_var_prefix(&self) -> String {
        format!("PROTO_{}", self.id.to_uppercase().replace('-', "_"))
    }

    /// Return an absolute path to the globals directory in which packages are installed to.
    pub fn get_globals_bin_dir(&self) -> Option<&Path> {
        self.globals_dir.as_deref()
    }

    /// Return a string that all globals are prefixed with. Will be used for filtering and listing.
    pub fn get_globals_prefix(&self) -> Option<&str> {
        self.globals_prefix.as_deref()
    }

    /// Return an absolute path to the tool's inventory directory. The inventory houses
    /// installed versions, the manifest, and more.
    pub fn get_inventory_dir(&self) -> PathBuf {
        if let Some(dir) = &self.metadata.inventory.override_dir {
            return dir.to_owned();
        }

        self.proto.tools_dir.join(self.id.as_str())
    }

    /// Return a human readable name for the tool.
    pub fn get_name(&self) -> &str {
        &self.metadata.name
    }

    /// Return the resolved version or "latest".
    pub fn get_resolved_version(&self) -> VersionSpec {
        self.version.clone().unwrap_or_default()
    }

    /// Return a path to a local shim file if it exists.
    pub fn get_shim_path(&self) -> Option<PathBuf> {
        let local = self
            .get_tool_dir()
            .join("shims")
            .join(get_shim_file_name(&self.id, false));

        if local.exists() {
            return Some(local);
        }

        None
    }

    /// Return an absolute path to a temp directory solely for this tool.
    pub fn get_temp_dir(&self) -> PathBuf {
        self.proto.temp_dir.join(self.id.as_str())
    }

    /// Return an absolute path to the tool's install directory for the currently resolved version.
    pub fn get_tool_dir(&self) -> PathBuf {
        let mut version = self.get_resolved_version().to_string();

        if let Some(suffix) = &self.metadata.inventory.version_suffix {
            version = format!("{}{}", version, suffix);
        }

        self.get_inventory_dir().join(version)
    }

    /// Explicitly set the version to use.
    pub fn set_version(&mut self, version: VersionSpec) {
        self.version = Some(version);
    }

    /// Disable progress bars when installing or uninstalling the tool.
    pub fn disable_progress_bars(&self) -> bool {
        self.metadata.inventory.disable_progress_bars
    }

    pub fn from_virtual_path(&self, path: &Path) -> PathBuf {
        self.plugin.from_virtual_path(path)
    }

    pub fn to_virtual_path(&self, path: &Path) -> VirtualPath {
        // This is a temporary hack. Only newer plugins support the `VirtualPath`
        // type, so we need to check if the plugin has a version or not, which
        // is a newer feature. Otherwise, old plugins would fail to parse the
        // `VirtualPath` type and crash.
        if self.metadata.plugin_version.is_some() {
            self.plugin.to_virtual_path(path)
        } else {
            match self.plugin.to_virtual_path(path) {
                VirtualPath::WithReal { path, .. } => VirtualPath::Only(path),
                VirtualPath::Only(path) => VirtualPath::Only(path),
            }
        }
    }
}

// APIs

impl Tool {
    /// Return contextual information to pass to WASM plugin functions.
    pub fn create_context(&self) -> miette::Result<ToolContext> {
        Ok(ToolContext {
            env_vars: self
                .metadata
                .env_vars
                .iter()
                .filter_map(|var| env::var(var).ok().map(|value| (var.to_owned(), value)))
                .collect(),
            tool_dir: self.to_virtual_path(&self.get_tool_dir()),
            version: self.get_resolved_version().to_string(),
        })
    }

    /// Register the tool by loading initial metadata and persisting it.
    pub fn register_tool(&mut self) -> miette::Result<()> {
        let mut metadata: ToolMetadataOutput = self.plugin.cache_func_with(
            "register_tool",
            ToolMetadataInput {
                id: self.id.to_string(),
            },
        )?;

        if let Some(override_dir) = &metadata.inventory.override_dir {
            let inventory_dir = self.from_virtual_path(override_dir);

            if inventory_dir.is_absolute() {
                metadata.inventory.override_dir = Some(inventory_dir);
            } else {
                return Err(ProtoError::AbsoluteInventoryDir.into());
            }
        }

        self.metadata = metadata;

        Ok(())
    }

    /// Run a hook with the provided name and input.
    pub fn run_hook<I>(&self, hook: &str, input: I) -> miette::Result<()>
    where
        I: Debug + Serialize,
    {
        if self.plugin.has_func(hook) {
            self.plugin.call_func_without_output(hook, input)?;
        }

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
                context: self.create_context()?,
            },
        )?;

        if sync_changes.skip_sync {
            return Ok(());
        }

        let mut modified = false;

        if let Some(default) = sync_changes.default_version {
            modified = true;

            self.manifest.default_version =
                Some(UnresolvedVersionSpec::parse(&default).map_err(|error| {
                    ProtoError::Semver {
                        version: default,
                        error,
                    }
                })?);
        }

        if let Some(versions) = sync_changes.versions {
            modified = true;

            let mut entries = BTreeMap::new();
            let mut installed = HashSet::new();

            for version in versions {
                let key = VersionSpec::Version(version);
                let value = self
                    .manifest
                    .versions
                    .get(&key)
                    .cloned()
                    .unwrap_or_default();

                installed.insert(key.clone());
                entries.insert(key, value);
            }

            self.manifest.versions = entries;
            self.manifest.installed_versions = installed;
        }

        if modified {
            self.manifest.save()?;
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

        // Don't use the overridden inventory path
        let cache_path = self
            .proto
            .tools_dir
            .join(self.id.as_str())
            .join("remote-versions.json");

        // Attempt to read from the cache first
        if cache_path.exists() && (is_cache_enabled() || is_offline()) {
            let metadata = fs::metadata(&cache_path)?;

            // If offline, always use the cache, otherwise only within the last 12 hours
            let read_cache = if is_offline() {
                true
            } else if !self.cache {
                false
            } else if let Ok(modified_time) = metadata.modified().or_else(|_| metadata.created()) {
                modified_time > SystemTime::now() - Duration::from_secs(60 * 60 * 12)
            } else {
                false
            };

            if read_cache {
                debug!(tool = self.id.as_str(), cache = ?cache_path, "Loading from local cache");

                versions = read_json_file_with_lock(&cache_path)?;
                cached = true;
            }
        }

        // Nothing cached, so load from the plugin
        if !cached {
            if is_offline() {
                return Err(ProtoError::InternetConnectionRequired.into());
            }

            versions = self.plugin.cache_func_with(
                "load_versions",
                LoadVersionsInput {
                    initial: initial_version.to_string(),
                    context: self.create_context()?,
                },
            )?;

            write_json_file_with_lock(cache_path, &versions)?;
        }

        // Cache the results and create a resolver
        let mut resolver = VersionResolver::from_output(versions);
        resolver.with_manifest(&self.manifest)?;

        Ok(resolver)
    }

    /// Given an initial version, resolve it to a fully qualifed and semantic version
    /// (or alias) according to the tool's ecosystem.
    pub async fn resolve_version(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
    ) -> miette::Result<()> {
        if self.version.is_some() {
            return Ok(());
        }

        debug!(
            tool = self.id.as_str(),
            initial_version = initial_version.to_string(),
            "Resolving a semantic version or alias",
        );

        // If offline but we have a fully qualified semantic version,
        // exit early and assume the version is legitimate! Additionally,
        // canary is a special type that we can simply just use.
        if is_offline() && matches!(initial_version, UnresolvedVersionSpec::Version(_))
            || matches!(initial_version, UnresolvedVersionSpec::Canary)
        {
            let version = initial_version.to_resolved_spec();

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

            self.version = Some(version);

            return Ok(());
        }

        let resolver = self.load_version_resolver(initial_version).await?;
        let mut version = VersionSpec::default();
        let mut resolved = false;

        if self.plugin.has_func("resolve_version") {
            let result: ResolveVersionOutput = self.plugin.call_func_with(
                "resolve_version",
                ResolveVersionInput {
                    initial: initial_version.to_string(),
                    context: self.create_context()?,
                },
            )?;

            if let Some(candidate) = result.candidate {
                debug!(
                    tool = self.id.as_str(),
                    candidate = &candidate,
                    "Received a possible version or alias to use",
                );

                resolved = true;
                version = resolver.resolve(&UnresolvedVersionSpec::parse(&candidate).map_err(
                    |error| ProtoError::Semver {
                        version: candidate,
                        error,
                    },
                )?)?;
            }

            if let Some(candidate) = result.version {
                debug!(
                    tool = self.id.as_str(),
                    version = &candidate,
                    "Received an explicit version or alias to use",
                );

                resolved = true;
                version = VersionSpec::parse(&candidate).map_err(|error| ProtoError::Semver {
                    version: candidate,
                    error,
                })?;
            }
        }

        if !resolved {
            version = resolver.resolve(initial_version)?;
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

        self.version = Some(version);

        Ok(())
    }
}

// VERSION DETECTION

impl Tool {
    /// Attempt to detect an applicable version from the provided directory.
    pub async fn detect_version_from(
        &self,
        current_dir: &Path,
    ) -> miette::Result<Option<UnresolvedVersionSpec>> {
        if !self.plugin.has_func("detect_version_files") {
            return Ok(None);
        }

        let has_parser = self.plugin.has_func("parse_version_file");
        let result: DetectVersionOutput = self.plugin.cache_func("detect_version_files")?;

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
                content
            };

            debug!(
                tool = self.id.as_str(),
                file = ?file_path,
                "Detected a version"
            );

            return Ok(Some(
                UnresolvedVersionSpec::parse(&version)
                    .map_err(|error| ProtoError::Semver { version, error })?,
            ));
        }

        Ok(None)
    }
}

// INSTALLATION

impl Tool {
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

        let mut verified = false;

        // Allow plugin to provide their own checksum verification method
        if self.plugin.has_func("verify_checksum") {
            let result: VerifyChecksumOutput = self.plugin.call_func_with(
                "verify_checksum",
                VerifyChecksumInput {
                    checksum_file: self.to_virtual_path(checksum_file),
                    download_file: self.to_virtual_path(download_file),
                    context: self.create_context()?,
                },
            )?;

            verified = result.verified;

        // Otherwise attempt to verify it ourselves
        } else {
            match checksum_file.extension().map(|e| e.to_str().unwrap()) {
                Some("minisig" | "minisign") => {
                    use minisign_verify::*;

                    let handle_error = |error: Error| ProtoError::Minisign { error };

                    let public_key = PublicKey::from_base64(
                        checksum_public_key.ok_or_else(|| ProtoError::MissingChecksumPublicKey)?,
                    )
                    .map_err(handle_error)?;

                    public_key
                        .verify(
                            &fs::read_file_bytes(download_file)?,
                            &Signature::decode(&fs::read_file(checksum_file)?)
                                .map_err(handle_error)?,
                            false,
                        )
                        .map_err(handle_error)?;

                    verified = true;
                }
                _ => {
                    let checksum_hash = hash_file_contents(download_file)?;
                    let checksum_matching_line =
                        format!("{}  {}", checksum_hash, fs::file_name(download_file));

                    for line in BufReader::new(fs::open_file(checksum_file)?)
                        .lines()
                        .flatten()
                    {
                        // <checksum>  <file>
                        // <checksum>
                        if line == checksum_matching_line || line == checksum_hash {
                            verified = true;
                            break;
                        }
                    }
                }
            };
        }

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

    pub async fn build_from_source(&self, install_dir: &Path) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool by building from source"
        );

        if !self.plugin.has_func("build_instructions") {
            return Err(ProtoError::UnsupportedBuildFromSource {
                tool: self.id.clone(),
            }
            .into());
        }

        let temp_dir = self
            .get_temp_dir()
            .join(self.get_resolved_version().to_string());

        let options: BuildInstructionsOutput = self.plugin.cache_func_with(
            "build_instructions",
            BuildInstructionsInput {
                context: self.create_context()?,
            },
        )?;

        match &options.source {
            // Should this do anything?
            SourceLocation::None => {
                return Ok(());
            }

            // Download from archive
            SourceLocation::Archive { url: archive_url } => {
                let download_file = temp_dir.join(extract_filename_from_url(archive_url)?);

                debug!(
                    tool = self.id.as_str(),
                    archive_url,
                    download_file = ?download_file,
                    install_dir = ?install_dir,
                    "Attempting to download and unpack sources",
                );

                download_from_url_to_file(
                    archive_url,
                    &download_file,
                    self.proto.get_http_client()?,
                )
                .await?;

                Archiver::new(install_dir, &download_file).unpack_from_ext()?;
            }

            // Clone from Git repository
            SourceLocation::Git {
                url: repo_url,
                reference: ref_name,
                submodules,
            } => {
                debug!(
                    tool = self.id.as_str(),
                    repo_url,
                    ref_name,
                    install_dir = ?install_dir,
                    "Attempting to clone a Git repository",
                );

                let run_git = |args: &[&str]| -> miette::Result<()> {
                    let status = Command::new("git")
                        .args(args)
                        .current_dir(install_dir)
                        .spawn()
                        .into_diagnostic()?
                        .wait()
                        .into_diagnostic()?;

                    if !status.success() {
                        return Err(ProtoError::BuildFailed {
                            url: repo_url.clone(),
                            status: format!("exit code {}", status),
                        }
                        .into());
                    }

                    Ok(())
                };

                // TODO, pull if already cloned

                fs::create_dir_all(install_dir)?;

                run_git(&[
                    "clone",
                    if *submodules {
                        "--recurse-submodules"
                    } else {
                        ""
                    },
                    repo_url,
                    ".",
                ])?;

                run_git(&["checkout", ref_name])?;
            }
        };

        Ok(())
    }

    /// Download the tool (as an archive) from its distribution registry
    /// into the `~/.proto/temp` folder, and optionally verify checksums.
    pub async fn install_from_prebuilt(&self, temp_dir: &Path) -> miette::Result<PathBuf> {
        debug!(
            tool = self.id.as_str(),
            "Installing tool from a pre-built archive"
        );

        let client = self.proto.get_http_client()?;
        let options: DownloadPrebuiltOutput = self.plugin.cache_func_with(
            "download_prebuilt",
            DownloadPrebuiltInput {
                context: self.create_context()?,
                install_dir: self.to_virtual_path(temp_dir),
            },
        )?;

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

            download_from_url_to_file(&download_url, &download_file, client).await?;
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

                download_from_url_to_file(&checksum_url, &checksum_file, client).await?;
            }

            self.verify_checksum(
                &checksum_file,
                &download_file,
                options.checksum_public_key.as_deref(),
            )
            .await?;

            fs::remove_file(checksum_file)?;
        }

        // Attempt to unpack the archive
        debug!(
            tool = self.id.as_str(),
            download_file = ?download_file,
            install_dir = ?temp_dir,
            "Attempting to unpack archive",
        );

        if self.plugin.has_func("unpack_archive") {
            self.plugin.call_func_without_output(
                "unpack_archive",
                UnpackArchiveInput {
                    input_file: self.to_virtual_path(&download_file),
                    output_dir: self.to_virtual_path(temp_dir),
                    context: self.create_context()?,
                },
            )?;

            // Is an archive, unpack it
        } else if is_archive_file(&download_file) {
            let mut archiver = Archiver::new(temp_dir, &download_file);

            if let Some(prefix) = &options.archive_prefix {
                archiver.set_prefix(prefix);
            }

            archiver.unpack_from_ext()?;

            // Not an archive, assume a binary and copy
        } else {
            let install_path = temp_dir.join(if cfg!(windows) {
                format!("{}.exe", self.id)
            } else {
                self.id.to_string()
            });

            fs::rename(&download_file, &install_path)?;
            fs::update_perms(install_path, None)?;
        }

        if download_file.exists() {
            fs::remove_file(&download_file)?;
        }

        Ok(download_file)
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

        let install_dir = self.get_tool_dir();
        let temp_install_dir =
            self.get_temp_dir()
                .join(format!("{}-{}", self.get_resolved_version(), now()));
        let mut installed = false;

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
                    context: self.create_context()?,
                    install_dir: self.to_virtual_path(&temp_install_dir),
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

            self.install_from_prebuilt(&temp_install_dir).await?;

            // Ensure the final destination does not exist before moving
            if install_dir.exists() {
                fs::remove_dir_all(&install_dir)?;
            }

            // Move the built/unpacked files to the final destination
            fs::rename(temp_install_dir, &install_dir)?;
        }

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

    /// Install a global dependency/package for the tool.
    pub async fn install_global(&self, dependency: &str) -> miette::Result<bool> {
        let globals_dir = self.get_globals_bin_dir();

        if !self.plugin.has_func("install_global") || globals_dir.is_none() {
            return Ok(false);
        }

        let result: InstallGlobalOutput = self.plugin.call_func_with(
            "install_global",
            InstallGlobalInput {
                dependency: dependency.to_owned(),
                globals_dir: self.to_virtual_path(globals_dir.as_ref().unwrap()),
                context: self.create_context()?,
            },
        )?;

        if !result.installed {
            return Err(ProtoError::InstallFailed {
                tool: dependency.to_owned(),
                error: result.error.unwrap_or_default(),
            }
            .into());
        }

        self.on_installed_global
            .emit(InstalledGlobalEvent {
                dependency: dependency.to_owned(),
            })
            .await?;

        Ok(result.installed)
    }

    /// Uninstall the tool by deleting the current install directory.
    pub async fn uninstall(&self) -> miette::Result<bool> {
        let install_dir = self.get_tool_dir();

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
                    context: self.create_context()?,
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

    /// Uninstall a global dependency/package from the tool.
    pub async fn uninstall_global(&self, dependency: &str) -> miette::Result<bool> {
        let globals_dir = self.get_globals_bin_dir();

        if !self.plugin.has_func("uninstall_global") || globals_dir.is_none() {
            return Ok(false);
        }

        let result: UninstallGlobalOutput = self.plugin.call_func_with(
            "uninstall_global",
            UninstallGlobalInput {
                dependency: dependency.to_owned(),
                globals_dir: self.to_virtual_path(globals_dir.as_ref().unwrap()),
                context: self.create_context()?,
            },
        )?;

        if !result.uninstalled {
            return Err(ProtoError::UninstallFailed {
                tool: dependency.to_owned(),
                error: result.error.unwrap_or_default(),
            }
            .into());
        }

        self.on_uninstalled_global
            .emit(UninstalledGlobalEvent {
                dependency: dependency.to_owned(),
            })
            .await?;

        Ok(result.uninstalled)
    }

    /// Find the absolute file path to the tool's binary that will be executed.
    pub async fn locate_bins(&mut self) -> miette::Result<()> {
        if self.bin_path.is_some() {
            return Ok(());
        }

        let mut options = LocateBinsOutput::default();
        let tool_dir = self.get_tool_dir();

        debug!(tool = self.id.as_str(), "Locating binaries for tool");

        if self.plugin.has_func("locate_bins") {
            options = self.plugin.cache_func_with(
                "locate_bins",
                LocateBinsInput {
                    context: self.create_context()?,
                },
            )?;
        }

        let bin_path = if let Some(bin) = options.bin_path {
            let bin = self.from_virtual_path(&bin);

            if bin.is_absolute() {
                bin
            } else {
                tool_dir.join(bin)
            }
        } else {
            tool_dir.join(self.id.as_str())
        };

        debug!(tool = self.id.as_str(), bin_path = ?bin_path, "Found a binary");

        if bin_path.exists() {
            self.bin_path = Some(bin_path);

            return Ok(());
        }

        Err(ProtoError::MissingToolBin {
            tool: self.get_name().to_owned(),
            bin: bin_path,
        }
        .into())
    }

    /// Find the directory global packages are installed to.
    pub async fn locate_globals_dir(&mut self) -> miette::Result<()> {
        if !self.plugin.has_func("locate_bins") || self.globals_dir.is_some() {
            return Ok(());
        }

        debug!(
            tool = self.id.as_str(),
            "Locating globals bin directory for tool"
        );

        let install_dir = self.get_tool_dir();
        let options: LocateBinsOutput = self.plugin.cache_func_with(
            "locate_bins",
            LocateBinsInput {
                context: self.create_context()?,
            },
        )?;

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

            if dir_path.exists() || (index == lookup_count && options.fallback_last_globals_dir) {
                debug!(tool = self.id.as_str(), bin_dir = ?dir_path, "Found a globals directory");

                self.globals_dir = Some(dir_path);
                break;
            }
        }

        Ok(())
    }
}

// SHIMMER

impl Tool {
    /// Create the context object required for creating shim files.
    pub fn create_shim_context(&self) -> ShimContext {
        ShimContext {
            shim_file: &self.id,
            bin: &self.id,
            tool_id: &self.id,
            tool_dir: Some(self.get_tool_dir()),
            tool_version: Some(self.get_resolved_version().to_string()),
            ..ShimContext::default()
        }
    }

    /// Create global and local shim files for the current tool.
    /// If find only is enabled, will only check if they exist, and not create.
    pub async fn create_shims(&self, find_only: bool) -> miette::Result<()> {
        let mut primary_context = self.create_shim_context();
        let mut shim_event = CreatedShimsEvent {
            global: vec![],
            local: vec![],
        };

        // If not configured from the plugin, always create the primary global
        if !self.plugin.has_func("create_shims") {
            create_global_shim(&self.proto, primary_context, find_only)?;

            shim_event.global.push(self.id.to_string());

            self.on_created_shims.emit(shim_event).await?;

            return Ok(());
        }

        let shim_configs: CreateShimsOutput = self.plugin.cache_func_with(
            "create_shims",
            CreateShimsInput {
                context: self.create_context()?,
            },
        )?;

        // Create the primary global shim
        if let Some(primary_config) = &shim_configs.primary {
            primary_context.before_args = primary_config.before_args.as_deref();
            primary_context.after_args = primary_config.after_args.as_deref();
        }

        if !shim_configs.no_primary_global {
            create_global_shim(&self.proto, primary_context, find_only)?;
            shim_event.global.push(self.id.to_string());
        }

        // Create alternate/secondary global shims
        for (name, config) in &shim_configs.global_shims {
            let mut context = self.create_shim_context();
            context.shim_file = name;
            context.bin_path = config.bin_path.as_deref();
            context.before_args = config.before_args.as_deref();
            context.after_args = config.after_args.as_deref();

            create_global_shim(&self.proto, context, find_only)?;
            shim_event.global.push(name.to_owned());
        }

        // Create local shims
        for (name, config) in &shim_configs.local_shims {
            let bin_path = if let Some(path) = &config.bin_path {
                self.get_tool_dir().join(path)
            } else {
                self.get_tool_dir().join(self.id.as_str())
            };

            let mut context = self.create_shim_context();
            context.shim_file = name;
            context.bin_path = Some(&bin_path);
            context.parent_bin = config.parent_bin.as_deref();
            context.before_args = config.before_args.as_deref();
            context.after_args = config.after_args.as_deref();

            create_local_shim(context, find_only)?;
            shim_event.local.push(name.to_owned());
        }

        self.on_created_shims.emit(shim_event).await?;

        Ok(())
    }
}

// OPERATIONS

impl Tool {
    /// Return true if the tool has been installed. This is less accurate than `is_setup`,
    /// as it only checks for the existence of the inventory directory.
    pub fn is_installed(&self) -> bool {
        let dir = self.get_tool_dir();

        self.version
            .as_ref()
            // Canary can be overwritten so treat as not-installed
            .is_some_and(|v| {
                !v.is_latest() && !v.is_canary() && self.manifest.installed_versions.contains(v)
            })
            && dir.exists()
            && !fs::is_dir_locked(dir)
    }

    /// Return true if the tool has been setup (installed and binaries are located).
    pub async fn is_setup(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
    ) -> miette::Result<bool> {
        self.resolve_version(initial_version).await?;

        let install_dir = self.get_tool_dir();

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

            if self.bin_path.is_none() {
                self.locate_bins().await?;
                self.setup_shims(false).await?;
                self.setup_bin_link(false)?;
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
        self.resolve_version(initial_version).await?;

        if self.install(build_from_source).await? {
            self.cleanup().await?;
            self.locate_bins().await?;

            // Always force create shims to ensure changes are propagated
            self.setup_shims(true).await?;

            // Only link on the first install or if the bin doesn't exist
            self.setup_bin_link(false)?;

            // Add version to manifest
            let mut default = None;

            if let Some(default_version) = &self.metadata.default_version {
                default = Some(
                    UnresolvedVersionSpec::parse(default_version).map_err(|error| {
                        ProtoError::Semver {
                            version: default_version.to_owned(),
                            error,
                        }
                    })?,
                );
            }

            self.manifest
                .insert_version(self.get_resolved_version(), default)?;

            // Allow plugins to override manifest
            self.sync_manifest()?;

            return Ok(true);
        }

        Ok(false)
    }

    /// Create a symlink from the current tool to the proto bin directory.
    pub fn setup_bin_link(&mut self, force: bool) -> miette::Result<()> {
        let input_path = self.get_bin_path()?;
        let output_path = self.proto.bin_dir.join(self.get_bin_name());

        if output_path.exists() && !force {
            return Ok(());
        }

        // Don't support other extensions on Windows at this time
        #[cfg(windows)]
        {
            if input_path.extension().is_some_and(|e| e != "exe") {
                return Ok(());
            }
        }

        debug!(
            tool = self.id.as_str(),
            source = ?input_path,
            target = ?output_path,
            "Creating a symlink to the original tool executable"
        );

        fs::remove_file(&output_path)?;
        fs::create_dir_all(&self.proto.bin_dir)?;

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(input_path, &output_path).into_diagnostic()?;
        }

        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(input_path, &output_path).into_diagnostic()?;
        }

        Ok(())
    }

    /// Setup shims if they are missing or out of date.
    pub async fn setup_shims(&mut self, force: bool) -> miette::Result<()> {
        let is_outdated = self.manifest.shim_version != SHIM_VERSION;
        let do_create = force || is_outdated || env::var("CI").is_ok();

        if do_create {
            debug!(
                tool = self.id.as_str(),
                "Creating shims as they either do not exist, or are outdated"
            );

            self.manifest.shim_version = SHIM_VERSION;
            self.manifest.save()?;
        }

        self.create_shims(!do_create).await?;

        Ok(())
    }

    /// Teardown the tool by uninstalling the current version, removing the version
    /// from the manifest, and cleaning up temporary files. Return true if the teardown occurred.
    pub async fn teardown(&mut self) -> miette::Result<bool> {
        self.cleanup().await?;

        if self.uninstall().await? {
            // Only remove if uninstall was successful
            self.manifest.remove_version(self.get_resolved_version())?;

            return Ok(true);
        }

        Ok(false)
    }

    /// Delete temporary files and downloads for the current version.
    pub async fn cleanup(&mut self) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Cleaning up temporary files and downloads"
        );

        fs::remove(self.get_temp_dir())?;

        Ok(())
    }
}
