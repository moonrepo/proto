use crate::error::ProtoError;
use crate::helpers::{get_proto_version, ENV_VAR};
use crate::layout::{Inventory, Product};
use crate::proto::ProtoEnvironment;
use crate::shim_registry::{Shim, ShimRegistry, ShimsMap};
use miette::IntoDiagnostic;
use proto_pdk_api::*;
use proto_shim::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::env;
use std::fmt::{self, Debug};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, instrument, warn};
use warpgate::{
    host::{create_host_functions, HostData},
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

    // Private
    pub(crate) cache: bool,
    pub(crate) exes_dir: Option<PathBuf>,
    pub(crate) exe_path: Option<PathBuf>,
    pub(crate) globals_dirs: Vec<PathBuf>,
    pub(crate) globals_prefix: Option<String>,
}

impl Tool {
    pub async fn new(
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
            exes_dir: None,
            exe_path: None,
            globals_dirs: vec![],
            globals_prefix: None,
            id,
            inventory: Inventory::default(),
            locator: None,
            metadata: ToolMetadataOutput::default(),
            plugin,
            product: Product::default(),
            proto,
            version: None,
        };

        tool.register_tool().await?;

        Ok(tool)
    }

    #[instrument(name = "new_tool", skip(proto, wasm))]
    pub async fn load<I: AsRef<Id> + Debug, P: AsRef<ProtoEnvironment>>(
        id: I,
        proto: P,
        wasm: Wasm,
    ) -> miette::Result<Self> {
        let proto = proto.as_ref();

        Self::load_from_manifest(id, proto, Self::create_plugin_manifest(proto, wasm)?).await
    }

    pub async fn load_from_manifest<I: AsRef<Id>, P: AsRef<ProtoEnvironment>>(
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
                    http_client: Arc::clone(proto.get_plugin_loader()?.get_client()?),
                    virtual_paths: proto.get_virtual_paths(),
                    working_dir: proto.cwd.clone(),
                }),
            )?),
        )
        .await
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

    async fn call_locate_executables(&self) -> miette::Result<LocateExecutablesOutput> {
        self.plugin
            .cache_func_with(
                "locate_executables",
                LocateExecutablesInput {
                    context: self.create_context(),
                },
            )
            .await
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
            proto_version: Some(get_proto_version().to_owned()),
            tool_dir: self.to_virtual_path(&self.get_product_dir()),
            version: self.get_resolved_version(),
        }
    }

    /// Register the tool by loading initial metadata and persisting it.
    #[instrument(skip_all)]
    pub async fn register_tool(&mut self) -> miette::Result<()> {
        let metadata: ToolMetadataOutput = self
            .plugin
            .cache_func_with(
                "register_tool",
                ToolMetadataInput {
                    id: self.id.to_string(),
                },
            )
            .await?;

        let mut inventory = self
            .proto
            .store
            .create_inventory(&self.id, &metadata.inventory)?;

        if let Some(override_dir) = &metadata.inventory.override_dir {
            let override_dir_path = override_dir.real_path();

            debug!(
                tool = self.id.as_str(),
                override_virtual = ?override_dir,
                override_real = ?override_dir_path,
                "Attempting to override inventory directory"
            );

            if override_dir_path.is_none()
                || override_dir_path.as_ref().is_some_and(|p| p.is_relative())
            {
                return Err(ProtoError::AbsoluteInventoryDir {
                    tool: metadata.name.clone(),
                    dir: override_dir_path.unwrap_or_else(|| PathBuf::from("<unknown>")),
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
    #[instrument(skip_all)]
    pub async fn sync_manifest(&mut self) -> miette::Result<()> {
        if !self.plugin.has_func("sync_manifest").await {
            return Ok(());
        }

        debug!(tool = self.id.as_str(), "Syncing manifest with changes");

        let sync_changes: SyncManifestOutput = self
            .plugin
            .call_func_with(
                "sync_manifest",
                SyncManifestInput {
                    context: self.create_context(),
                },
            )
            .await?;

        if sync_changes.skip_sync {
            return Ok(());
        }

        let mut modified = false;
        let manifest = &mut self.inventory.manifest;

        if let Some(versions) = sync_changes.versions {
            modified = true;

            let mut entries = FxHashMap::default();
            let mut installed = FxHashSet::default();

            for key in versions {
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

// BINARIES, SHIMS

impl Tool {
    /// Create all executables for the current tool.
    /// - Locate the primary binary to execute.
    /// - Generate shims to `~/.proto/shims`.
    /// - Symlink bins to `~/.proto/bin`.
    #[instrument(skip(self))]
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

    /// Return an absolute path to the pre-installed executables directory.s
    pub fn get_exes_dir(&self) -> Option<&PathBuf> {
        self.exes_dir.as_ref()
    }

    /// Return an absolute path to the globals directory that actually exists.
    pub fn get_globals_dir(&self) -> Option<&PathBuf> {
        let lookup_count = self.globals_dirs.len() - 1;

        for (index, dir) in self.globals_dirs.iter().enumerate() {
            if dir.exists() || index == lookup_count {
                debug!(tool = self.id.as_str(), dir = ?dir, "Found a usable globals directory");

                return Some(dir);
            }
        }

        None
    }

    /// Return a list of all possible globals directories.
    pub fn get_globals_dirs(&self) -> &[PathBuf] {
        &self.globals_dirs
    }

    /// Return a string that all globals are prefixed with. Will be used for filtering and listing.
    pub fn get_globals_prefix(&self) -> Option<&str> {
        self.globals_prefix.as_deref()
    }

    /// Return a list of all binaries that get created in `~/.proto/bin`.
    /// The list will contain the executable config, and an absolute path
    /// to the binaries final location.
    pub async fn get_bin_locations(&self) -> miette::Result<Vec<ExecutableLocation>> {
        let options = self.call_locate_executables().await?;
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
    pub async fn get_exe_location(&self) -> miette::Result<Option<ExecutableLocation>> {
        let options = self.call_locate_executables().await?;

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
    pub async fn get_shim_locations(&self) -> miette::Result<Vec<ExecutableLocation>> {
        let options = self.call_locate_executables().await?;
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
    #[instrument(skip_all)]
    pub async fn locate_executable(&mut self) -> miette::Result<()> {
        debug!(tool = self.id.as_str(), "Locating executable for tool");

        let exe_path = if let Some(location) = self.get_exe_location().await? {
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

    /// Locate the directory that local executables are installed to.
    #[instrument(skip_all)]
    pub async fn locate_exes_dir(&mut self) -> miette::Result<()> {
        if !self.plugin.has_func("locate_executables").await || self.exes_dir.is_some() {
            return Ok(());
        }

        let options = self.call_locate_executables().await?;

        if let Some(exes_dir) = options.exes_dir {
            self.exes_dir = Some(self.get_product_dir().join(exes_dir));
        }

        Ok(())
    }

    /// Locate the directories that global packages are installed to.
    #[instrument(skip_all)]
    pub async fn locate_globals_dirs(&mut self) -> miette::Result<()> {
        if !self.plugin.has_func("locate_executables").await || !self.globals_dirs.is_empty() {
            return Ok(());
        }

        debug!(
            tool = self.id.as_str(),
            "Locating globals bin directories for tool"
        );

        let install_dir = self.get_product_dir();
        let options = self.call_locate_executables().await?;

        self.globals_prefix = options.globals_prefix;

        // Find all possible global directories that packages can be installed to
        let mut resolved_dirs = vec![];

        'outer: for dir_lookup in options.globals_lookup_dirs {
            let mut dir = dir_lookup.clone();

            // If a lookup contains an env var, find and replace it.
            // If the var is not defined or is empty, skip this lookup.
            for cap in ENV_VAR.captures_iter(&dir_lookup) {
                let find_by = cap.get(0).unwrap().as_str();

                let replace_with = match find_by {
                    "$CWD" | "$PWD" => self.proto.cwd.clone(),
                    "$HOME" => self.proto.home.clone(),
                    "$PROTO_HOME" | "$PROTO_ROOT" => self.proto.root.clone(),
                    "$TOOL_DIR" => install_dir.clone(),
                    _ => match env::var_os(cap.get(1).unwrap().as_str()) {
                        Some(value) => PathBuf::from(value),
                        None => {
                            continue 'outer;
                        }
                    },
                };

                if let Some(replacement) = replace_with.to_str() {
                    dir = dir.replace(find_by, replacement);
                } else {
                    continue 'outer;
                }
            }

            let dir = if let Some(dir_suffix) = dir.strip_prefix('~') {
                self.proto.home.join(dir_suffix)
            } else {
                PathBuf::from(dir)
            };

            // Don't use a set as we need to persist the order!
            if !resolved_dirs.contains(&dir) {
                resolved_dirs.push(dir);
            }
        }

        debug!(
            tool = self.id.as_str(),
            dirs = ?resolved_dirs,
            "Located possible globals directories",
        );

        self.globals_dirs = resolved_dirs;

        Ok(())
    }

    /// Create shim files for the current tool if they are missing or out of date.
    /// If find only is enabled, will only check if they exist, and not create.
    #[instrument(skip(self))]
    pub async fn generate_shims(&mut self, force: bool) -> miette::Result<()> {
        let shims = self.get_shim_locations().await?;

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

        let mut registry: ShimsMap = BTreeMap::default();
        registry.insert(self.id.to_string(), Shim::default());

        let mut to_create = vec![];

        for shim in shims {
            let mut shim_entry = Shim::default();

            // Handle before and after args
            if let Some(before_args) = shim.config.shim_before_args {
                shim_entry.before_args = match before_args {
                    StringOrVec::String(value) => shell_words::split(&value).into_diagnostic()?,
                    StringOrVec::Vec(value) => value,
                };
            }

            if let Some(after_args) = shim.config.shim_after_args {
                shim_entry.after_args = match after_args {
                    StringOrVec::String(value) => shell_words::split(&value).into_diagnostic()?,
                    StringOrVec::Vec(value) => value,
                };
            }

            if let Some(env_vars) = shim.config.shim_env_vars {
                shim_entry.env_vars.extend(env_vars);
            }

            if !shim.primary {
                shim_entry.parent = Some(self.id.to_string());

                // Only use --alt when the secondary executable exists
                if shim.config.exe_path.is_some() {
                    shim_entry.alt_bin = Some(true);
                }
            }

            // Create the shim file by copying the source bin
            if force_create || find_only && !shim.path.exists() {
                to_create.push(shim.path);
            }

            // Update the registry
            registry.insert(shim.name.clone(), shim_entry);
        }

        // Only lock the directory and create shims if necessary
        if !to_create.is_empty() {
            let _lock = fs::lock_directory(&self.proto.store.shims_dir)?;

            for shim_path in to_create {
                self.proto.store.create_shim(&shim_path)?;

                debug!(
                    tool = self.id.as_str(),
                    shim = ?shim_path,
                    shim_version = SHIM_VERSION,
                    "Creating shim"
                );
            }

            ShimRegistry::update(&self.proto, registry)?;
        }

        Ok(())
    }

    /// Symlink all primary and secondary binaries for the current tool.
    #[instrument(skip(self))]
    pub async fn symlink_bins(&mut self, force: bool) -> miette::Result<()> {
        let bins = self.get_bin_locations().await?;

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

        let tool_dir = self.get_product_dir();
        let mut to_create = vec![];

        for bin in bins {
            let input_path = tool_dir.join(
                bin.config
                    .exe_link_path
                    .as_ref()
                    .or(bin.config.exe_path.as_ref())
                    .unwrap(),
            );
            let output_path = bin.path;

            if !input_path.exists() {
                warn!(
                    tool = self.id.as_str(),
                    source = ?input_path,
                    target = ?output_path,
                    "Unable to symlink binary, source file does not exist"
                );

                continue;
            }

            if !force && output_path.exists() {
                continue;
            }

            to_create.push((input_path, output_path));
        }

        // Only lock the directory and create bins if necessary
        if !to_create.is_empty() {
            let _lock = fs::lock_directory(&self.proto.store.bin_dir)?;

            for (input_path, output_path) in to_create {
                debug!(
                    tool = self.id.as_str(),
                    source = ?input_path,
                    target = ?output_path,
                    "Creating binary symlink"
                );

                self.proto.store.unlink_bin(&output_path)?;
                self.proto.store.link_bin(&output_path, &input_path)?;
            }
        }

        Ok(())
    }
}

impl fmt::Debug for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tool")
            .field("id", &self.id)
            .field("metadata", &self.metadata)
            .field("locator", &self.locator)
            .field("proto", &self.proto)
            .field("version", &self.version)
            .field("inventory", &self.inventory)
            .field("product", &self.product)
            .finish()
    }
}
