use crate::{errors::ProtoError, helpers::get_root, plugin::PluginLocator};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::toml;
use std::env;

pub const USER_CONFIG_NAME: &str = "config.toml";

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserConfig {
    pub auto_clean: bool,
    pub auto_install: bool,
    pub node_intercept_globals: bool,
    pub plugins: FxHashMap<String, PluginLocator>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            auto_clean: false,
            auto_install: false,
            node_intercept_globals: true,
            plugins: FxHashMap::default(),
        }
    }
}

impl UserConfig {
    #[tracing::instrument(skip_all)]
    pub fn load() -> Result<Self, ProtoError> {
        let path = get_root()?.join(USER_CONFIG_NAME);

        if !path.exists() {
            return Ok(UserConfig::default());
        }

        let config: UserConfig = toml::read_file(&path)?;

        Ok(config)
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            auto_clean: env::var("PROTO_AUTO_CLEAN").is_ok(),
            auto_install: env::var("PROTO_AUTO_INSTALL").is_ok(),
            plugins: FxHashMap::default(),
        }
    }
}
