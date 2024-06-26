use crate::error::ProtoError;
use crate::helpers::is_offline;
use crate::layout::Store;
use crate::proto_config::{ProtoConfig, ProtoConfigFile, ProtoConfigManager, PROTO_CONFIG_NAME};
use once_cell::sync::OnceCell;
use starbase_utils::dirs::home_dir;
use starbase_utils::fs;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;
use warpgate::PluginLoader;

#[derive(Clone, Default)]
pub struct ProtoEnvironment {
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
        let home = get_home_dir()?;
        let root = match env::var("PROTO_HOME") {
            Ok(root) => root.into(),
            Err(_) => home.join(".proto"),
        };

        Self::from(root, home)
    }

    pub fn new_testing(sandbox: &Path) -> miette::Result<Self> {
        let mut env = Self::from(sandbox.join(".proto"), sandbox.join(".home")).unwrap();
        env.cwd = sandbox.to_path_buf();
        env.test_only = true;

        // Paths must exist for things to work correctly!
        fs::create_dir_all(&env.root)?;
        fs::create_dir_all(&env.home)?;

        Ok(env)
    }

    pub fn from<P: AsRef<Path>, H: AsRef<Path>>(root: P, home: H) -> miette::Result<Self> {
        let root = root.as_ref();

        debug!(store = ?root, "Creating proto environment, detecting store");

        Ok(ProtoEnvironment {
            cwd: env::current_dir().expect("Unable to determine current working directory!"),
            env_mode: env::var("PROTO_ENV").ok(),
            home: home.as_ref().to_owned(),
            root: root.to_owned(),
            config_manager: Arc::new(OnceCell::new()),
            plugin_loader: Arc::new(OnceCell::new()),
            test_only: env::var("PROTO_TEST").is_ok(),
            store: Store::new(root),
        })
    }

    pub fn get_config_dir(&self, global: bool) -> &Path {
        if global {
            &self.root
        } else {
            &self.cwd
        }
    }

    pub fn get_plugin_loader(&self) -> miette::Result<&PluginLoader> {
        let config = self.load_config()?;

        self.plugin_loader.get_or_try_init(|| {
            let mut loader = PluginLoader::new(&self.store.plugins_dir, &self.store.temp_dir);
            loader.set_client_options(&config.settings.http);
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

    #[tracing::instrument(name = "load_all", skip_all)]
    pub fn load_config(&self) -> miette::Result<&ProtoConfig> {
        self.load_config_manager()?.get_merged_config()
    }

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
            .field("cwd", &self.cwd)
            .field("env_mode", &self.env_mode)
            .field("home", &self.home)
            .field("root", &self.root)
            .field("store", &self.store)
            .field("test_only", &self.test_only)
            .finish()
    }
}

fn get_home_dir() -> miette::Result<PathBuf> {
    Ok(home_dir().ok_or(ProtoError::MissingHomeDir)?)
}
