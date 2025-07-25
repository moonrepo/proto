use crate::env::ProtoEnvironment;
use crate::helpers::get_proto_version;
use crate::layout::{Inventory, Product};
use crate::lockfile::LockRecord;
use crate::tool_error::ProtoToolError;
use crate::tool_spec::Backend;
use proto_pdk_api::*;
use schematic::ConfigEnum;
use starbase_styles::color;
use std::fmt::{self, Debug};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};
use warpgate::{
    Id, PluginContainer, PluginLocator, PluginManifest, VirtualPath, Wasm,
    host::{HostData, create_host_functions},
};

pub type ToolMetadata = RegisterToolOutput;

pub struct Tool {
    pub backend: Option<Backend>,
    pub id: Id,
    pub locator: Option<PluginLocator>,
    pub metadata: ToolMetadata,
    pub plugin: Arc<PluginContainer>,
    pub proto: Arc<ProtoEnvironment>,
    pub version: Option<VersionSpec>,

    // Store
    pub inventory: Inventory,
    pub product: Product,

    // Cache
    pub(crate) backend_registered: bool,
    pub(crate) cache: bool,
    pub(crate) exe_file: Option<PathBuf>,
    pub(crate) exes_dirs: Vec<PathBuf>,
    pub(crate) globals_dir: Option<PathBuf>,
    pub(crate) globals_dirs: Vec<PathBuf>,
    pub(crate) globals_prefix: Option<String>,
    pub(crate) version_locked: Option<LockRecord>,
}

impl Tool {
    pub async fn new(
        id: Id,
        proto: Arc<ProtoEnvironment>,
        plugin: Arc<PluginContainer>,
    ) -> Result<Self, ProtoToolError> {
        debug!(
            "Created tool {} and its WASM runtime",
            color::id(id.as_str())
        );

        let mut tool = Tool {
            backend: None,
            backend_registered: false,
            cache: true,
            exe_file: None,
            exes_dirs: vec![],
            globals_dir: None,
            globals_dirs: vec![],
            globals_prefix: None,
            id,
            inventory: Inventory::default(),
            locator: None,
            metadata: ToolMetadata::default(),
            plugin,
            product: Product::default(),
            proto,
            version: None,
            version_locked: None,
        };

        tool.register_tool().await?;

        Ok(tool)
    }

    #[instrument(name = "new_tool", skip(proto, wasm))]
    pub async fn load<I: AsRef<Id> + fmt::Debug, P: AsRef<ProtoEnvironment>>(
        id: I,
        proto: P,
        wasm: Wasm,
    ) -> Result<Self, ProtoToolError> {
        let proto = proto.as_ref();

        Self::load_from_manifest(id, proto, Self::create_plugin_manifest(proto, wasm)?).await
    }

    pub async fn load_from_manifest<I: AsRef<Id>, P: AsRef<ProtoEnvironment>>(
        id: I,
        proto: P,
        manifest: PluginManifest,
    ) -> Result<Self, ProtoToolError> {
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
                    cache_dir: proto.store.cache_dir.clone(),
                    http_client: Arc::clone(proto.get_plugin_loader()?.get_http_client()?),
                    virtual_paths: proto.get_virtual_paths(),
                    working_dir: proto.working_dir.clone(),
                }),
            )?),
        )
        .await
    }

    pub fn create_plugin_manifest<P: AsRef<ProtoEnvironment>>(
        proto: P,
        wasm: Wasm,
    ) -> Result<PluginManifest, ProtoToolError> {
        let proto = proto.as_ref();

        let mut manifest = PluginManifest::new([wasm]);
        manifest = manifest.with_allowed_host("*");
        manifest = manifest.with_allowed_paths(proto.get_virtual_paths_compat().into_iter());
        // manifest = manifest.with_timeout(Duration::from_secs(90));

        #[cfg(debug_assertions)]
        {
            use std::time::Duration;

            manifest = manifest.with_timeout(Duration::from_secs(300));
        }

        Ok(manifest)
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

    /// Return true if this tool instance is a backend plugin.
    pub async fn is_backend_plugin(&self) -> bool {
        if self.plugin.has_func("register_backend").await {
            Backend::variants()
                .iter()
                .any(|var| var.to_string() == self.id.as_str())
        } else {
            false
        }
    }

    /// Explicitly set the version to use.
    pub fn set_version(&mut self, version: VersionSpec) {
        self.product = self.inventory.create_product(&version);
        self.version = Some(version);
    }

    /// Convert a virtual path to a real path.
    pub fn from_virtual_path(&self, path: impl AsRef<Path> + Debug) -> PathBuf {
        self.plugin.from_virtual_path(path)
    }

    /// Convert a real path to a virtual path.
    pub fn to_virtual_path(&self, path: impl AsRef<Path> + Debug) -> VirtualPath {
        self.plugin.to_virtual_path(path)
    }
}

// APIs

impl Tool {
    /// Return contextual information to pass to WASM plugin functions.
    pub fn create_context(&self) -> ToolContext {
        ToolContext {
            proto_version: Some(get_proto_version().to_owned()),
            temp_dir: self.to_virtual_path(self.get_temp_dir()),
            tool_dir: self.to_virtual_path(self.get_product_dir()),
            version: self.get_resolved_version(),
        }
    }

    /// Return contextual information to pass to WASM plugin functions,
    /// representing an unresolved state, which has no version or tool
    /// data.
    pub fn create_unresolved_context(&self) -> ToolUnresolvedContext {
        ToolUnresolvedContext {
            proto_version: Some(get_proto_version().to_owned()),
            temp_dir: self.to_virtual_path(&self.inventory.temp_dir),
            // version: self.version.clone(),
            // TODO: temporary until 3rd-party plugins update their PDKs
            tool_dir: self.to_virtual_path(&self.proto.store.inventory_dir),
            version: self
                .version
                .clone()
                .or_else(|| Some(VersionSpec::Alias("latest".into()))),
        }
    }

    /// Register the tool by loading initial metadata and persisting it.
    #[instrument(skip_all)]
    pub async fn register_tool(&mut self) -> Result<(), ProtoToolError> {
        let metadata: RegisterToolOutput = self
            .plugin
            .cache_func_with(
                "register_tool",
                RegisterToolInput {
                    id: self.id.to_string(),
                },
            )
            .await?;

        #[cfg(not(debug_assertions))]
        if let Some(expected_version) = &metadata.minimum_proto_version {
            let actual_version = get_proto_version();

            if actual_version < expected_version {
                return Err(ProtoToolError::InvalidMinimumVersion {
                    tool: metadata.name,
                    id: self.id.clone(),
                    expected: expected_version.to_string(),
                    actual: actual_version.to_string(),
                }
                .into());
            }
        }

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
                return Err(ProtoToolError::RequiredAbsoluteInventoryDir {
                    tool: metadata.name.clone(),
                    dir: override_dir_path.unwrap_or_else(|| PathBuf::from("<unknown>")),
                });
            }

            inventory.dir_original = Some(inventory.dir);
            inventory.dir = self.from_virtual_path(override_dir);
        }

        self.product = inventory.create_product(&self.get_resolved_version());
        self.inventory = inventory;
        self.metadata = metadata;

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
