use crate::error::ProtoError;
use crate::helpers::is_offline;
use crate::layout::Store;
use crate::proto_config::{
    ConfigMode, PinLocation, ProtoConfig, ProtoConfigFile, ProtoConfigManager, PROTO_CONFIG_NAME,
};
use once_cell::sync::OnceCell;
use starbase_utils::dirs::home_dir;
use starbase_utils::env::path_var;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;
use warpgate::PluginLoader;

#[derive(Clone, Default)]
pub struct ProtoEnvironment {
    pub config_mode: ConfigMode,
    pub cwd: PathBuf,
    pub env_mode: Option<String>,
    pub home: PathBuf, // ~
    pub root: PathBuf, // ~/.proto
    pub store: Store,
    pub test_only: bool,

    config_manager: Arc<OnceCell<ProtoConfigManager>>,
    plugin_loader: Arc<OnceCell<PluginLoader>>,
}

impl ProtoEnvironment {
    pub fn new() -> miette::Result<Self> {
        let home = home_dir().ok_or(ProtoError::MissingHomeDir)?;
        let mut root = path_var("PROTO_HOME").unwrap_or_else(|| home.join(".proto"));

        if let Ok(rel_root) = root.strip_prefix("~") {
            root = home.join(rel_root);
        }

        Self::from(root, home)
    }

    pub fn new_testing(sandbox: &Path) -> miette::Result<Self> {
        let mut env = Self::from(sandbox.join(".proto"), sandbox.join(".home"))?;
        env.test_only = true;

        Ok(env)
    }

    pub fn from<R: AsRef<Path>, H: AsRef<Path>>(root: R, home: H) -> miette::Result<Self> {
        let root = root.as_ref();
        let home = home.as_ref();

        debug!(
            store = ?root,
            home = ?home,
            "Creating proto environment, detecting store",
        );

        Ok(ProtoEnvironment {
            config_mode: ConfigMode::Upwards,
            cwd: env::current_dir().expect("Unable to determine current working directory!"),
            env_mode: env::var("PROTO_ENV").ok(),
            home: home.to_owned(),
            root: root.to_owned(),
            config_manager: Arc::new(OnceCell::new()),
            plugin_loader: Arc::new(OnceCell::new()),
            test_only: env::var("PROTO_TEST").is_ok(),
            store: Store::new(root),
        })
    }

    pub fn get_config_dir(&self, pin: PinLocation) -> &Path {
        match pin {
            PinLocation::Global => &self.root,
            PinLocation::Local => &self.cwd,
            PinLocation::User => &self.home,
        }
    }

    pub fn get_plugin_loader(&self) -> miette::Result<&PluginLoader> {
        let config = self.load_config()?;

        self.plugin_loader.get_or_try_init(|| {
            let mut options = config.settings.http.clone();
            options.cache_dir = Some(self.store.cache_dir.join("requests"));

            let mut loader = PluginLoader::new(&self.store.plugins_dir, &self.store.temp_dir);
            loader.set_client_options(&options);
            loader.set_offline_checker(is_offline);

            Ok(loader)
        })
    }

    pub fn get_virtual_paths(&self) -> BTreeMap<PathBuf, PathBuf> {
        BTreeMap::from_iter([
            (self.cwd.clone(), "/cwd".into()),
            (self.root.clone(), "/proto".into()),
            (self.home.clone(), "/userhome".into()),
        ])
    }

    pub fn get_virtual_paths_compat(&self) -> BTreeMap<String, PathBuf> {
        self.get_virtual_paths()
            .into_iter()
            .map(|(key, value)| (key.to_string_lossy().to_string(), value))
            .collect()
    }

    pub fn load_config(&self) -> miette::Result<&ProtoConfig> {
        self.load_config_with_mode(self.config_mode)
    }

    pub fn load_config_with_mode(&self, mode: ConfigMode) -> miette::Result<&ProtoConfig> {
        let manager = self.load_config_manager()?;

        match mode {
            ConfigMode::Global => manager.get_global_config(),
            ConfigMode::Local => manager.get_local_config(&self.cwd),
            ConfigMode::Upwards => manager.get_merged_config_without_global(),
            ConfigMode::UpwardsGlobal => manager.get_merged_config(),
        }
    }

    #[tracing::instrument(name = "load_all", skip_all)]
    pub fn load_config_manager(&self) -> miette::Result<&ProtoConfigManager> {
        self.config_manager.get_or_try_init(|| {
            // Don't traverse passed the home directory,
            // but only if working directory is within it!
            let end_dir = if self.cwd.starts_with(&self.home) {
                Some(self.home.as_path())
            } else {
                None
            };

            let mut manager = ProtoConfigManager::load(&self.cwd, end_dir, self.env_mode.as_ref())?;

            // Always load the proto home/root config last
            let path = self.root.join(PROTO_CONFIG_NAME);

            manager.files.push(ProtoConfigFile {
                exists: path.exists(),
                global: true,
                path,
                config: ProtoConfig::load_from(&self.root, true)?,
            });

            Ok(manager)
        })
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
            .field("cwd", &self.cwd)
            .field("env_mode", &self.env_mode)
            .field("home", &self.home)
            .field("root", &self.root)
            .field("store", &self.store)
            .field("test_only", &self.test_only)
            .finish()
    }
}
