use crate::config::{ConfigMode, PROTO_CONFIG_NAME, PinLocation, PluginType, ProtoConfig};
use crate::config_error::ProtoConfigError;
use crate::config_trust::{TrustState, TrustStore, get_sensitive_fields};
use crate::env_error::ProtoEnvError;
use crate::file_manager::{
    ProtoConfigFile, ProtoDirEntry, ProtoFileManager, TrustedPluginRecord, UntrustedPluginRecord,
};
use crate::helpers::is_offline;
use crate::layout::Store;
use crate::lockfile::ProtoLock;
use crate::registry::ProtoRegistry;
use crate::telemetry::MetricTimer;
use crate::tool_context::ToolContext;
use once_cell::sync::OnceCell;
use proto_pdk_api::sort_paths_list;
use starbase_utils::dirs::home_dir;
use starbase_utils::{envx, string_vec};
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use system_env::{SystemDetails, SystemPlatform};
use toml_edit::DocumentMut;
use tracing::{debug, instrument};
use warpgate::{PluginLoader, PluginLocator};

#[derive(Clone, Default)]
pub struct ProtoEnvironment {
    pub config_mode: ConfigMode,
    pub env_mode: Option<String>,
    pub home_dir: PathBuf, // ~
    pub otel_enabled: bool,
    pub store: Store,
    pub test_only: bool,
    pub trust: TrustStore,
    pub working_dir: PathBuf,

    file_manager: Arc<OnceCell<ProtoFileManager>>,
    foreign_platform: bool,
    host_details: Arc<OnceCell<SystemDetails>>,
    host_platform: Arc<OnceCell<SystemPlatform>>,
    plugin_loader: Arc<OnceCell<PluginLoader>>,
    registry: Arc<OnceCell<ProtoRegistry>>,
}

impl ProtoEnvironment {
    pub fn new() -> Result<Self, ProtoEnvError> {
        let home = home_dir().ok_or(ProtoEnvError::MissingHomeDir)?;
        let mut root = envx::path_var("PROTO_HOME")
            .or_else(|| envx::path_var("XDG_DATA_HOME").map(|xdg| xdg.join("proto")))
            .unwrap_or_else(|| home.join(".proto"));

        if let Ok(rel_root) = root.strip_prefix("~") {
            root = home.join(rel_root);
        }

        Self::from(root, home)
    }

    pub fn new_testing(sandbox: &Path) -> Result<Self, ProtoEnvError> {
        let mut env = Self::from(sandbox.join(".proto"), sandbox.join(".home"))?;
        env.test_only = true;

        // Tests write configs throughout the sandbox, so trust it, unless trusted
        // paths are explicitly configured (even if empty). CI is ignored so that
        // tests behave the same everywhere.
        env.trust.trust_all = false;

        if env::var_os("PROTO_TRUSTED_PATHS").is_none() {
            env.trust.add_trusted_path(sandbox);
        }

        Ok(env)
    }

    pub fn from<R: AsRef<Path>, H: AsRef<Path>>(root: R, home: H) -> Result<Self, ProtoEnvError> {
        let root = root.as_ref();
        let home = home.as_ref();

        debug!(
            store = ?root,
            home = ?home,
            "Creating proto environment, detecting store",
        );

        Ok(ProtoEnvironment {
            config_mode: ConfigMode::Upwards,
            working_dir: env::current_dir().map_err(|_| ProtoEnvError::MissingWorkingDir)?,
            env_mode: env::var("PROTO_ENV").ok(),
            home_dir: home.to_owned(),
            otel_enabled: false,
            file_manager: Arc::new(OnceCell::new()),
            foreign_platform: false,
            host_details: Arc::new(OnceCell::new()),
            host_platform: Arc::new(OnceCell::new()),
            plugin_loader: Arc::new(OnceCell::new()),
            registry: Arc::new(OnceCell::new()),
            test_only: env::var("PROTO_TEST").is_ok(),
            trust: TrustStore::new(root.join("trust")),
            store: Store::new(root),
        })
    }

    pub fn create_metric(&self) -> MetricTimer {
        MetricTimer::start(self.otel_enabled)
    }

    pub fn get_config_dir(&self, pin: PinLocation) -> Result<&Path, ProtoConfigError> {
        match pin {
            PinLocation::Closest => {
                // Only project directories, never the user or global configs.
                // Pins are always written to the base config, so ignore
                // directories that only have an environment config
                for entry in &self.load_file_manager()?.entries {
                    if entry.location == PinLocation::Local
                        && entry.configs.iter().any(|file| {
                            file.exists
                                && file
                                    .path
                                    .file_name()
                                    .is_some_and(|name| name == PROTO_CONFIG_NAME)
                        })
                    {
                        return Ok(&entry.path);
                    }
                }

                Ok(&self.working_dir)
            }
            PinLocation::Global => Ok(&self.store.dir),
            PinLocation::Local => Ok(&self.working_dir),
            PinLocation::User => Ok(&self.home_dir),
        }
    }

    /// Return the platform (architecture, operating system, and libc) of the
    /// host machine. Detecting the libc requires executing a command on Linux,
    /// so detection is deferred until first requested.
    pub fn get_host_platform(&self) -> &SystemPlatform {
        self.host_platform.get_or_init(SystemPlatform::from_env)
    }

    pub fn set_host_platform(&mut self, platform: SystemPlatform) {
        self.host_platform = Arc::new(OnceCell::with_value(platform));
    }

    /// Return details about the hardware and operating system of the host
    /// machine, like the x86-64 microarchitecture level, or none when targeting
    /// another platform, as they only describe the host machine. Detection is
    /// deferred until first requested, and shared with copies of this environment.
    pub fn get_host_details(&self) -> Option<&SystemDetails> {
        if self.foreign_platform {
            return None;
        }

        Some(
            self.host_details
                .get_or_init(|| SystemDetails::from_platform(self.get_host_platform())),
        )
    }

    /// Return a copy of this environment that targets the provided platform
    /// instead of the host machine. Tools loaded with the copy receive the
    /// platform as their plugin's host environment, and create lock records
    /// for it, while config, lockfile, and plugin state remain shared.
    ///
    /// When the platform differs from the host machine's, the environment is
    /// foreign: plugins receive `HostEnvironment.foreign` as true, and no host
    /// details. Plugins can still execute commands (warpgate logs a warning),
    /// but the results only reflect the host machine, not the platform.
    pub fn for_platform(&self, platform: SystemPlatform) -> Self {
        let mut env = self.clone();
        env.foreign_platform = self.foreign_platform || &platform != self.get_host_platform();
        env.set_host_platform(platform);
        env
    }

    pub fn is_foreign_platform(&self) -> bool {
        self.foreign_platform
    }

    pub fn get_plugin_loader(&self) -> Result<&PluginLoader, ProtoConfigError> {
        let config = self.load_config()?;

        self.plugin_loader.get_or_try_init(|| {
            let mut options = config.settings.http.clone();
            options.cache_dir = Some(self.store.cache_dir.join("requests-v2"));

            let mut loader =
                PluginLoader::new(&self.store.plugins_dir, &self.store.plugins_temp_dir);

            if let Some(secs) = config.settings.cache_duration {
                loader.set_cache_duration(Duration::from_secs(secs));
            }

            loader.set_http_client_options(&options);
            loader.set_offline_checker(is_offline);
            loader.add_extensions(string_vec!["toml", "json", "jsonc", "yaml", "yml"]);
            loader.add_registries(config.settings.registries.iter().cloned().collect());

            Ok(loader)
        })
    }

    pub fn load_registry(&self) -> &ProtoRegistry {
        self.registry
            .get_or_init(|| ProtoRegistry::new(self.store.clone()))
    }

    pub fn get_virtual_paths(&self) -> Vec<(PathBuf, PathBuf)> {
        let mut paths = vec![
            (self.store.temp_dir.clone(), "/temp".into()),
            (self.store.dir.clone(), "/proto".into()),
            (self.home_dir.clone(), "/userhome".into()),
        ];

        // This is required for situtations where users are using proto
        // outside of the home directory, and the WASM plugin will need
        // access to it!
        if self.test_only
            || (!self.working_dir.starts_with(&self.home_dir)
                && !paths.iter().any(|(path, _)| path == &self.working_dir))
        {
            paths.push((
                self.working_dir.clone(),
                if self.test_only {
                    "/sandbox".into()
                } else {
                    "/cwd".into()
                },
            ));
        }

        sort_paths_list(&mut paths);

        paths
    }

    pub fn load_config(&self) -> Result<&ProtoConfig, ProtoConfigError> {
        self.load_config_with_mode(self.config_mode)
    }

    pub fn load_config_with_mode(
        &self,
        mode: ConfigMode,
    ) -> Result<&ProtoConfig, ProtoConfigError> {
        let manager = self.load_file_manager()?;

        match mode {
            ConfigMode::Global => manager.get_global_config(),
            ConfigMode::Local => manager.get_local_config(&self.working_dir),
            ConfigMode::Upwards => manager.get_merged_config_without_global(),
            ConfigMode::UpwardsGlobal => manager.get_merged_config(),
        }
    }

    /// Return true if the config at the provided path is owned by the user, and
    /// never requires trust. Only the user and global configs are.
    pub fn is_config_owned_by_user(&self, config_path: &Path) -> bool {
        config_path
            .parent()
            .is_some_and(|dir| dir == self.home_dir || dir == self.store.dir)
    }

    /// Update a config file through its document, like [`ProtoConfig::update_document`].
    ///
    /// If the config had no security-sensitive settings before the update, but
    /// has some after, the user introduced them through proto (for example with
    /// `proto plugin add`), so the file is trusted. A config that was already
    /// untrusted stays untrusted.
    pub fn update_config_document<P: AsRef<Path> + fmt::Debug, F: FnOnce(&mut DocumentMut)>(
        &self,
        dir: P,
        op: F,
    ) -> Result<PathBuf, ProtoConfigError> {
        let path = ProtoConfig::resolve_path(dir);

        if self.is_config_owned_by_user(&path) || self.trust.is_trusted(&path) {
            return ProtoConfig::update_document(&path, op);
        }

        // Inspect the content within the lock held by the update, so that a
        // concurrent change to the file is never trusted by mistake. Content
        // that fails to parse is treated as requiring trust.
        let requires_trust = |doc: &DocumentMut| {
            !ProtoConfig::parse_content(doc.to_string(), &path)
                .and_then(|config| get_sensitive_fields(&config))
                .is_ok_and(|fields| fields.is_empty())
        };

        let mut required_before = true;
        let mut required_after = false;

        let path = ProtoConfig::update_document(&path, |doc| {
            required_before = requires_trust(doc);
            op(doc);
            required_after = requires_trust(doc);
        })?;

        if !required_before && required_after {
            debug!(
                config = ?path,
                "Trusting config, as security-sensitive settings were added through proto",
            );

            self.trust.trust(&path)?;
        }

        Ok(path)
    }

    pub fn load_config_files(&self) -> Result<Vec<&ProtoConfigFile>, ProtoConfigError> {
        Ok(self
            .load_file_manager()?
            .entries
            .iter()
            .filter_map(|dir| {
                if !self.config_mode.includes_global() && dir.location == PinLocation::Global
                    || self.config_mode.only_local() && dir.path != self.working_dir
                    || self.config_mode.only_global() && dir.path != self.store.dir
                {
                    None
                } else {
                    Some(&dir.configs)
                }
            })
            .flatten()
            .collect())
    }

    /// Return the plugin that would have been loaded for the provided tool, if
    /// its config had been trusted. Configs are ordered by precedence, so
    /// returns `None` when a closer config has configured an applied plugin,
    /// as trusting a config further away wouldn't change what's loaded.
    pub fn get_untrusted_plugin(
        &self,
        context: &ToolContext,
        ty: PluginType,
    ) -> Result<Option<UntrustedPluginRecord>, ProtoConfigError> {
        for file in self.load_config_files()? {
            if let Some(locator) = file.get_untrusted_plugin(context, ty) {
                return Ok(Some(UntrustedPluginRecord {
                    config: file.path.clone(),
                    context: context.to_owned(),
                    locator: locator.to_owned(),
                    type_of: ty,
                }));
            }

            if !file.get_plugins(context, ty).is_empty() {
                return Ok(None);
            }
        }

        Ok(None)
    }

    /// Return where the plugin that was loaded for the provided tool came from.
    /// Accepts the loaded locator instead of finding it, as a plugin may not be
    /// configured at all, and isn't always from the closest config: a plugin in
    /// `[tools.*]` takes precedence over `[plugins.tools]` in a closer config.
    pub fn get_trusted_plugin(
        &self,
        context: &ToolContext,
        ty: PluginType,
        locator: &PluginLocator,
    ) -> Result<TrustedPluginRecord, ProtoConfigError> {
        let builtin_plugins = self.load_config()?.builtin_plugins();
        let builtin = builtin_plugins.get(context.backend.as_ref().unwrap_or(&context.id), ty);

        let mut config = None;
        let mut overrides = vec![];

        for file in self.load_config_files()? {
            for plugin in file.get_plugins(context, ty) {
                if plugin == locator {
                    config.get_or_insert_with(|| file.path.clone());
                } else if !overrides.contains(plugin) {
                    overrides.push(plugin.to_owned());
                }
            }
        }

        // Built-in plugins are only file locators in debug builds, while
        // the same file configured explicitly is not a replacement
        let debug_file = match locator {
            PluginLocator::File(file) if config.is_none() && builtin == Some(locator) => {
                Some(file.get_resolved_path())
            }
            _ => None,
        };

        if let Some(builtin) = builtin
            && builtin != locator
            && !overrides.contains(builtin)
        {
            overrides.push(builtin.to_owned());
        }

        Ok(TrustedPluginRecord {
            config,
            context: context.to_owned(),
            debug_file,
            locator: locator.to_owned(),
            overrides,
            type_of: ty,
        })
    }

    pub fn load_lock(
        &self,
        context: &ToolContext,
    ) -> Result<Option<RwLockReadGuard<'_, ProtoLock>>, ProtoConfigError> {
        let manager = self.load_file_manager()?;

        match manager.get_locked_config(context) {
            Some(file) => manager.get_lock(&file.path),
            None => Ok(None),
        }
    }

    pub fn load_lock_mut(
        &self,
        context: &ToolContext,
    ) -> Result<Option<RwLockWriteGuard<'_, ProtoLock>>, ProtoConfigError> {
        let manager = self.load_file_manager()?;

        match manager.get_locked_config(context) {
            Some(file) => manager.get_lock_mut(&file.path),
            None => Ok(None),
        }
    }

    #[instrument(skip_all)]
    pub fn load_file_manager(&self) -> Result<&ProtoFileManager, ProtoConfigError> {
        self.file_manager.get_or_try_init(|| {
            // Don't traverse passed the home directory,
            // but only if working directory is within it!
            let end_dir = if self.working_dir.starts_with(&self.home_dir) {
                Some(self.home_dir.as_path())
            } else {
                None
            };

            let mut manager =
                ProtoFileManager::load(&self.working_dir, end_dir, self.env_mode.as_ref())?;

            // Always load the proto home/root config last
            let path = self.store.dir.join(PROTO_CONFIG_NAME);

            manager.entries.push(ProtoDirEntry {
                path: self.store.dir.clone(),
                location: PinLocation::Global,
                configs: vec![ProtoConfigFile {
                    exists: path.exists(),
                    path,
                    config: ProtoConfig::load_from(&self.store.dir, true)?,
                    locked: false,
                    trust: TrustState::NotRequired,
                    sensitive: vec![],
                    untrusted_config: None,
                }],
            });

            // Remove the pinned `proto` version from global/user configs,
            // as it causes massive recursion and `proto` process chains
            manager.remove_proto_pins();

            // Remove security-sensitive settings from untrusted configs
            manager.apply_trust(&self.trust, |path| self.is_config_owned_by_user(path));

            Ok(manager)
        })
    }

    pub fn reload(&self) -> Self {
        Self {
            config_mode: self.config_mode,
            env_mode: self.env_mode.clone(),
            home_dir: self.home_dir.clone(),
            otel_enabled: self.otel_enabled,
            store: self.store.clone(),
            test_only: self.test_only,
            trust: self.trust.clone(),
            working_dir: self.working_dir.clone(),
            file_manager: Arc::new(OnceCell::new()),
            foreign_platform: self.foreign_platform,
            host_details: self.host_details.clone(),
            host_platform: self.host_platform.clone(),
            plugin_loader: Arc::new(OnceCell::new()),
            registry: Arc::new(OnceCell::new()),
        }
    }
}

impl AsRef<ProtoEnvironment> for ProtoEnvironment {
    fn as_ref(&self) -> &ProtoEnvironment {
        self
    }
}

impl fmt::Debug for ProtoEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProtoEnvironment")
            .field("config_mode", &self.config_mode)
            .field("env_mode", &self.env_mode)
            .field("foreign_platform", &self.foreign_platform)
            .field("home_dir", &self.home_dir)
            .field("otel_enabled", &self.otel_enabled)
            .field("store", &self.store)
            .field("test_only", &self.test_only)
            .field("trust", &self.trust)
            .field("working_dir", &self.working_dir)
            .finish()
    }
}
