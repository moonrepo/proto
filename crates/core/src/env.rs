use crate::config::{ConfigMode, PROTO_CONFIG_NAME, PinLocation, ProtoConfig};
use crate::config_error::ProtoConfigError;
use crate::env_error::ProtoEnvError;
use crate::file_manager::{ProtoConfigFile, ProtoDirEntry, ProtoFileManager};
use crate::helpers::is_offline;
use crate::layout::Store;
use crate::lockfile::ProtoLock;
use once_cell::sync::OnceCell;
use starbase_console::{Console, EmptyReporter};
use starbase_utils::dirs::home_dir;
use starbase_utils::env::path_var;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use tracing::debug;
use warpgate::PluginLoader;

pub type ProtoConsole = Console<EmptyReporter>;

#[derive(Clone, Default)]
pub struct ProtoEnvironment {
    pub config_mode: ConfigMode,
    pub env_mode: Option<String>,
    pub home_dir: PathBuf, // ~
    pub store: Store,
    pub test_only: bool,
    pub working_dir: PathBuf,

    file_manager: Arc<OnceCell<ProtoFileManager>>,
    plugin_loader: Arc<OnceCell<PluginLoader>>,
}

impl ProtoEnvironment {
    pub fn new() -> Result<Self, ProtoEnvError> {
        let home = home_dir().ok_or(ProtoEnvError::MissingHomeDir)?;
        let mut root = path_var("PROTO_HOME")
            .or_else(|| path_var("XDG_DATA_HOME").map(|xdg| xdg.join("proto")))
            .unwrap_or_else(|| home.join(".proto"));

        if let Ok(rel_root) = root.strip_prefix("~") {
            root = home.join(rel_root);
        }

        Self::from(root, home)
    }

    pub fn new_testing(sandbox: &Path) -> Result<Self, ProtoEnvError> {
        let mut env = Self::from(sandbox.join(".proto"), sandbox.join(".home"))?;
        env.test_only = true;

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
            file_manager: Arc::new(OnceCell::new()),
            plugin_loader: Arc::new(OnceCell::new()),
            test_only: env::var("PROTO_TEST").is_ok(),
            store: Store::new(root),
        })
    }

    pub fn get_config_dir(&self, pin: PinLocation) -> &Path {
        match pin {
            PinLocation::Global => &self.store.dir,
            PinLocation::Local => &self.working_dir,
            PinLocation::User => &self.home_dir,
        }
    }

    pub fn get_plugin_loader(&self) -> Result<&PluginLoader, ProtoConfigError> {
        let config = self.load_config()?;

        self.plugin_loader.get_or_try_init(|| {
            let mut options = config.settings.http.clone();
            options.cache_dir = Some(self.store.cache_dir.join("requests"));

            let mut loader =
                PluginLoader::new(&self.store.plugins_dir, self.store.temp_dir.join("plugins"));

            if let Some(registries) = config.settings.registries.clone() {
                loader.add_registries(registries);
            };

            if let Some(secs) = config.settings.cache_duration {
                loader.set_cache_duration(Duration::from_secs(secs));
            }

            loader.set_client_options(&options);
            loader.set_offline_checker(is_offline);

            Ok(loader)
        })
    }

    pub fn get_virtual_paths(&self) -> BTreeMap<PathBuf, PathBuf> {
        BTreeMap::from_iter([
            (self.working_dir.clone(), "/cwd".into()),
            (self.store.dir.clone(), "/proto".into()),
            (self.home_dir.clone(), "/userhome".into()),
        ])
    }

    pub fn get_virtual_paths_compat(&self) -> BTreeMap<String, PathBuf> {
        self.get_virtual_paths()
            .into_iter()
            .map(|(key, value)| (key.to_string_lossy().to_string(), value))
            .collect()
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

    pub fn load_config_files(&self) -> Result<Vec<&ProtoConfigFile>, ProtoConfigError> {
        Ok(self
            .load_file_manager()?
            .entries
            .iter()
            .filter_map(|dir| {
                if !self.config_mode.includes_global() && dir.location == PinLocation::Global
                    || self.config_mode.only_local() && dir.path != self.working_dir
                {
                    None
                } else {
                    Some(&dir.configs)
                }
            })
            .flatten()
            .collect())
    }

    pub fn load_lock(&self) -> Result<Option<RwLockReadGuard<ProtoLock>>, ProtoConfigError> {
        Ok(self.load_file_manager()?.get_lock())
    }

    pub fn load_lock_mut(&self) -> Result<Option<RwLockWriteGuard<ProtoLock>>, ProtoConfigError> {
        Ok(self.load_file_manager()?.get_lock_mut())
    }

    #[tracing::instrument(name = "load_all", skip_all)]
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
                }],
                locked: false,
            });

            // Remove the pinned `proto` version from global/user configs,
            // as it causes massive recursion and `proto` process chains
            manager.remove_proto_pins();

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
            .field("env_mode", &self.env_mode)
            .field("home_dir", &self.home_dir)
            .field("store", &self.store)
            .field("test_only", &self.test_only)
            .field("working_dir", &self.working_dir)
            .finish()
    }
}
