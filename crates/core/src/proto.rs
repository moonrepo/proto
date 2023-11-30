use crate::helpers::{get_home_dir, get_proto_home, is_offline};
use crate::proto_config::{ProtoConfig, ProtoConfigManager, PROTO_CONFIG_ROOT_NAME};
use once_cell::sync::OnceCell;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use warpgate::{create_http_client_with_options, PluginLoader};

#[derive(Clone)]
pub struct ProtoEnvironment {
    pub bin_dir: PathBuf,
    pub cwd: PathBuf,
    pub plugins_dir: PathBuf,
    pub shims_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub home: PathBuf, // ~
    pub root: PathBuf, // ~/.proto

    config_manager: Arc<OnceCell<ProtoConfigManager>>,
    http_client: Arc<OnceCell<reqwest::Client>>,
    plugin_loader: Arc<OnceCell<PluginLoader>>,
    test_mode: bool,
}

impl ProtoEnvironment {
    pub fn new() -> miette::Result<Self> {
        Self::from(get_proto_home()?)
    }

    pub fn new_testing(sandbox: &Path) -> Self {
        let mut env = Self::from(sandbox.join(".proto")).unwrap();
        env.cwd = sandbox.to_path_buf();
        env.home = sandbox.join(".home");
        env.test_mode = true;
        env
    }

    pub fn from<P: AsRef<Path>>(root: P) -> miette::Result<Self> {
        let root = root.as_ref();

        Ok(ProtoEnvironment {
            bin_dir: root.join("bin"),
            cwd: env::current_dir().expect("Unable to determine current working directory!"),
            plugins_dir: root.join("plugins"),
            shims_dir: root.join("shims"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
            home: get_home_dir()?,
            root: root.to_owned(),
            config_manager: Arc::new(OnceCell::new()),
            http_client: Arc::new(OnceCell::new()),
            plugin_loader: Arc::new(OnceCell::new()),
            test_mode: false,
        })
    }

    pub fn get_http_client(&self) -> miette::Result<&reqwest::Client> {
        let config = self.load_config()?;

        self.http_client
            .get_or_try_init(|| create_http_client_with_options(&config.settings.http))
    }

    pub fn get_plugin_loader(&self) -> &PluginLoader {
        self.plugin_loader.get_or_init(|| {
            let mut loader = PluginLoader::new(&self.plugins_dir, &self.temp_dir);
            loader.set_offline_checker(is_offline);
            loader.set_seed(env!("CARGO_PKG_VERSION"));
            loader
        })
    }

    pub fn get_virtual_paths(&self) -> BTreeMap<PathBuf, PathBuf> {
        BTreeMap::from_iter([
            (self.cwd.clone(), "/workspace".into()),
            (self.root.clone(), "/proto".into()),
            (self.home.clone(), "/userhome".into()),
        ])
    }

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

            let mut manager = ProtoConfigManager::load(&self.cwd, end_dir)?;

            // Always load the proto home/root config
            manager.files.insert(
                PathBuf::from(PROTO_CONFIG_ROOT_NAME),
                ProtoConfigManager::load_from(&self.root)?,
            );

            Ok(manager)
        })
    }
}

impl AsRef<ProtoEnvironment> for ProtoEnvironment {
    fn as_ref(&self) -> &ProtoEnvironment {
        self
    }
}
