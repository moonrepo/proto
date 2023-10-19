use crate::helpers::{get_home_dir, get_proto_home};
use crate::user_config::UserConfig;
use once_cell::sync::OnceCell;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use warpgate::{create_http_client_with_options, PluginLoader};

#[derive(Clone, Debug)]
pub struct ProtoEnvironment {
    pub bin_dir: PathBuf,
    pub cwd: PathBuf,
    pub plugins_dir: PathBuf,
    pub shims_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub home: PathBuf, // ~
    pub root: PathBuf, // ~/.proto

    client: Arc<OnceCell<reqwest::Client>>,
    loader: Arc<OnceCell<PluginLoader>>,
}

impl ProtoEnvironment {
    pub fn new() -> miette::Result<Self> {
        Self::from(get_proto_home()?)
    }

    pub fn new_testing(sandbox: &Path) -> Self {
        let mut env = Self::from(sandbox.join(".proto")).unwrap();
        env.cwd = sandbox.to_path_buf();
        env.home = sandbox.join(".home");
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
            client: Arc::new(OnceCell::new()),
            loader: Arc::new(OnceCell::new()),
        })
    }

    pub fn get_http_client(&self) -> miette::Result<&reqwest::Client> {
        self.client.get_or_try_init(|| {
            let user_config = UserConfig::load()?;
            let client = create_http_client_with_options(user_config.http)?;

            Ok(client)
        })
    }

    pub fn get_plugin_loader(&self) -> &PluginLoader {
        self.loader.get_or_init(|| {
            let mut loader = PluginLoader::new(&self.plugins_dir, &self.temp_dir);
            loader.set_seed(env!("CARGO_PKG_VERSION"));
            loader
        })
    }

    pub fn get_user_config(&self) -> miette::Result<UserConfig> {
        UserConfig::load_from(&self.root)
    }
}

impl AsRef<ProtoEnvironment> for ProtoEnvironment {
    fn as_ref(&self) -> &ProtoEnvironment {
        self
    }
}
