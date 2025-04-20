use crate::config::{ConfigMode, PROTO_CONFIG_NAME, PinLocation, ProtoConfig};
use crate::config_manager::{ProtoConfigManager, ProtoFiles};
use crate::env_error::ProtoEnvError;
use crate::helpers::is_offline;
use crate::layout::Store;
use crate::lockfile::ProtoLockfile;
use once_cell::sync::OnceCell;
use starbase_console::{Console, EmptyReporter};
use starbase_utils::dirs::home_dir;
use starbase_utils::env::path_var;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

    config_manager: Arc<OnceCell<ProtoConfigManager>>,
    plugin_loader: Arc<OnceCell<PluginLoader>>,
}

impl ProtoEnvironment {
    pub fn new() -> miette::Result<Self> {
        let home = home_dir().ok_or(ProtoEnvError::MissingHomeDir)?;
        let mut root = path_var("PROTO_HOME")
            .or_else(|| path_var("XDG_DATA_HOME").map(|xdg| xdg.join("proto")))
            .unwrap_or_else(|| home.join(".proto"));

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
            working_dir: env::current_dir().map_err(|_| ProtoEnvError::MissingWorkingDir)?,
            env_mode: env::var("PROTO_ENV").ok(),
            home_dir: home.to_owned(),
            config_manager: Arc::new(OnceCell::new()),
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

    pub fn get_plugin_loader(&self) -> miette::Result<&PluginLoader> {
        let config = self.load_config()?;

        self.plugin_loader.get_or_try_init(|| {
            let mut options = config.settings.http.clone();
            options.cache_dir = Some(self.store.cache_dir.join("requests"));

            let mut loader =
                PluginLoader::new(&self.store.plugins_dir, self.store.temp_dir.join("plugins"));
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

    pub fn load_config(&self) -> miette::Result<&ProtoConfig> {
        self.load_config_with_mode(self.config_mode)
    }

    pub fn load_config_with_mode(&self, mode: ConfigMode) -> miette::Result<&ProtoConfig> {
        let manager = self.load_config_manager()?;

        match mode {
            ConfigMode::Global => manager.get_global_config(),
            ConfigMode::Local => manager.get_local_config(&self.working_dir),
            ConfigMode::Upwards => manager.get_merged_config_without_global(),
            ConfigMode::UpwardsGlobal => manager.get_merged_config(),
        }
    }

    pub fn load_config_files(&self) -> miette::Result<Vec<&ProtoFiles>> {
        Ok(self
            .load_config_manager()?
            .files
            .iter()
            .filter(|file| {
                !(!self.config_mode.includes_global() && file.location == PinLocation::Global
                    || self.config_mode.only_local()
                        && file
                            .config_path
                            .parent()
                            .is_none_or(|p| p != self.working_dir))
            })
            .collect())
    }

    #[tracing::instrument(name = "load_all_configs", skip_all)]
    pub fn load_config_manager(&self) -> miette::Result<&ProtoConfigManager> {
        self.config_manager.get_or_try_init(|| {
            // Don't traverse passed the home directory,
            // but only if working directory is within it!
            let end_dir = if self.working_dir.starts_with(&self.home_dir) {
                Some(self.home_dir.as_path())
            } else {
                None
            };

            let mut manager =
                ProtoConfigManager::load(&self.working_dir, end_dir, self.env_mode.as_ref())?;

            // Always load the proto home/root config last
            let path = self.store.dir.join(PROTO_CONFIG_NAME);

            manager.files.push(ProtoFiles {
                exists: path.exists(),
                location: PinLocation::Global,
                lockfile: None,
                lockfile_path: None,
                config_path: path,
                config: ProtoConfig::load_from(&self.store.dir)?,
            });

            // Remove the pinned `proto` version from global/user configs,
            // as it causes massive recursion and `proto` process chains
            manager.remove_proto_pins();

            Ok(manager)
        })
    }

    pub fn load_lockfile(&self) -> miette::Result<Option<&ProtoLockfile>> {
        self.load_lockfile_with_mode(self.config_mode)
    }

    pub fn load_lockfile_with_mode(
        &self,
        mode: ConfigMode,
    ) -> miette::Result<Option<&ProtoLockfile>> {
        let manager = self.load_config_manager()?;

        Ok(match mode {
            ConfigMode::Global => None,
            ConfigMode::Local => Some(manager.get_local_lockfile(&self.working_dir)?),
            _ => Some(manager.get_merged_lockfile()?),
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
