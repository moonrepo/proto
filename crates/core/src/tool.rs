use crate::env::ProtoEnvironment;
use crate::helpers::get_proto_version;
use crate::layout::{Inventory, Product};
use crate::lockfile::LockRecord;
use crate::normalize_path_separators;
use crate::tool_context::ToolContext;
use crate::tool_error::ProtoToolError;
use crate::utils::{archive, git};
use proto_pdk_api::*;
use starbase_styles::color;
use starbase_utils::fs;
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
    pub context: ToolContext,
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
        context: ToolContext,
        proto: Arc<ProtoEnvironment>,
        plugin: Arc<PluginContainer>,
    ) -> Result<Self, ProtoToolError> {
        debug!(
            "Created tool {} and its WASM runtime",
            color::id(context.as_str())
        );

        let mut tool = Tool {
            backend_registered: false,
            cache: true,
            context,
            exe_file: None,
            exes_dirs: vec![],
            globals_dir: None,
            globals_dirs: vec![],
            globals_prefix: None,
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

        if tool.context.backend.is_some() {
            tool.register_backend().await?;
        }

        Ok(tool)
    }

    #[instrument(name = "new_tool", skip(proto, wasm))]
    pub async fn load<I: AsRef<ToolContext> + fmt::Debug, P: AsRef<ProtoEnvironment>>(
        context: I,
        proto: P,
        wasm: Wasm,
    ) -> Result<Self, ProtoToolError> {
        let proto = proto.as_ref();

        Self::load_from_manifest(context, proto, Self::create_plugin_manifest(proto, wasm)?).await
    }

    pub async fn load_from_manifest<I: AsRef<ToolContext>, P: AsRef<ProtoEnvironment>>(
        context: I,
        proto: P,
        manifest: PluginManifest,
    ) -> Result<Self, ProtoToolError> {
        let context = context.as_ref();
        let proto = proto.as_ref();

        debug!(
            "Creating tool {} and instantiating plugin",
            color::id(context.as_str())
        );

        Self::new(
            context.to_owned(),
            Arc::new(proto.to_owned()),
            Arc::new(PluginContainer::new(
                context.id.clone(),
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

    /// Return the tool identifier.
    pub fn get_backend(&self) -> Option<&Id> {
        self.context.backend.as_ref()
    }

    /// Return the prefix for environment variable names.
    pub fn get_env_var_prefix(&self) -> String {
        format!("PROTO_{}", self.get_id().to_uppercase().replace('-', "_"))
    }

    /// Return the tool identifier.
    pub fn get_id(&self) -> &Id {
        &self.context.id
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
        self.plugin.has_func(PluginFunction::RegisterBackend).await
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
    pub fn create_plugin_context(&self) -> PluginContext {
        PluginContext {
            proto_version: Some(get_proto_version().to_owned()),
            temp_dir: self.to_virtual_path(self.get_temp_dir()),
            tool_dir: self.to_virtual_path(self.get_product_dir()),
            version: self.get_resolved_version(),
        }
    }

    /// Return contextual information to pass to WASM plugin functions,
    /// representing an unresolved state, which has no version or tool
    /// data.
    pub fn create_plugin_unresolved_context(&self) -> PluginUnresolvedContext {
        PluginUnresolvedContext {
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
                PluginFunction::RegisterTool,
                RegisterToolInput {
                    id: self.get_id().to_string(),
                },
            )
            .await?;

        #[cfg(not(debug_assertions))]
        if let Some(expected_version) = &metadata.minimum_proto_version {
            let actual_version = get_proto_version();

            if actual_version < expected_version {
                return Err(ProtoToolError::InvalidMinimumVersion {
                    tool: metadata.name,
                    id: self.get_id().clone(),
                    expected: expected_version.to_string(),
                    actual: actual_version.to_string(),
                }
                .into());
            }
        }

        let mut inventory = self
            .proto
            .store
            .create_inventory(self.get_id(), &metadata.inventory)?;

        if let Some(override_dir) = &metadata.inventory.override_dir {
            let override_dir_path = override_dir.real_path();

            debug!(
                tool = self.context.as_str(),
                override_virtual = ?override_dir,
                override_real = ?override_dir_path,
                "Attempting to override inventory directory"
            );

            if override_dir_path.as_ref().is_none_or(|p| p.is_relative()) {
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

    /// Register the backend by acquiring necessary source files.
    #[instrument(skip_all)]
    pub async fn register_backend(&mut self) -> Result<(), ProtoToolError> {
        if !self.plugin.has_func(PluginFunction::RegisterBackend).await || self.backend_registered {
            return Ok(());
        }

        let Some(backend) = &self.context.backend else {
            return Ok(());
        };

        let metadata: RegisterBackendOutput = self
            .plugin
            .cache_func_with(
                PluginFunction::RegisterBackend,
                RegisterBackendInput {
                    context: self.create_plugin_unresolved_context(),
                    id: self.get_id().to_string(),
                },
            )
            .await?;

        let Some(source) = metadata.source else {
            self.backend_registered = true;

            return Ok(());
        };

        let backend_id = metadata.backend_id;
        let backend_dir = self
            .proto
            .store
            .backends_dir
            .join(backend.to_string()) // asdf
            .join(&backend_id); // node
        let update_perms = !backend_dir.exists();
        let config = self.proto.load_config()?;

        // if is_offline() {
        //     return Err(ProtoEnvError::RequiredInternetConnection.into());
        // }

        debug!(
            tool = self.context.as_str(),
            backend_id,
            backend_dir = ?backend_dir,
            "Acquiring backend sources",
        );

        match source {
            SourceLocation::Archive(mut src) => {
                if !backend_dir.exists() {
                    src.url = config.rewrite_url(src.url);

                    debug!(
                        tool = self.context.as_str(),
                        url = &src.url,
                        "Downloading backend archive",
                    );

                    archive::download_and_unpack(
                        &src,
                        &backend_dir,
                        &self.proto.store.temp_dir,
                        self.proto
                            .get_plugin_loader()?
                            .get_http_client()?
                            .to_inner(),
                    )
                    .await?;
                }
            }
            SourceLocation::Git(src) => {
                debug!(
                    tool = self.context.as_str(),
                    url = &src.url,
                    "Cloning backend repository",
                );

                git::clone_or_pull_repo(&src, &backend_dir).await?;
            }
        };

        if update_perms {
            for exe in metadata.exes {
                let exe_path = backend_dir.join(normalize_path_separators(exe));

                if exe_path.exists() {
                    fs::update_perms(exe_path, None)?;
                }
            }
        }

        self.backend_registered = true;

        Ok(())
    }
}

impl fmt::Debug for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tool")
            .field("id", self.get_id())
            .field("metadata", &self.metadata)
            .field("locator", &self.locator)
            .field("proto", &self.proto)
            .field("version", &self.version)
            .field("inventory", &self.inventory)
            .field("product", &self.product)
            .finish()
    }
}
