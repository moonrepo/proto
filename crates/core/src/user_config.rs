use crate::{errors::ProtoError, helpers::get_root};
use rustc_hash::FxHashMap;
use serde::Deserialize;
use starbase_utils::toml;
use std::env;
use warpgate::PluginLocator;

pub const USER_CONFIG_NAME: &str = "config.toml";

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserConfig {
    pub auto_clean: bool,
    pub auto_install: bool,
    pub node_intercept_globals: bool,
    pub plugins: FxHashMap<String, PluginLocator>,
}

impl UserConfig {
    #[tracing::instrument(skip_all)]
    pub fn load() -> Result<Self, ProtoError> {
        let root = get_root()?;
        let path = root.join(USER_CONFIG_NAME);

        if !path.exists() {
            return Ok(UserConfig::default());
        }

        let mut config: UserConfig = toml::read_file(&path)?;

        // Update plugin file paths to be absolute
        for locator in config.plugins.values_mut() {
            if let PluginLocator::SourceFile {
                path: ref mut source_path,
                ..
            } = locator
            {
                *source_path = root.join(&source_path);
            }
        }

        Ok(config)
    }
}

fn from_var(name: &str, fallback: bool) -> bool {
    if let Ok(value) = env::var(name) {
        return value == "1" || value == "true" || value == "on";
    }

    fallback
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            auto_clean: from_var("PROTO_AUTO_CLEAN", false),
            auto_install: from_var("PROTO_AUTO_INSTALL", false),
            node_intercept_globals: from_var("PROTO_NODE_INTERCEPT_GLOBALS", true),
            plugins: FxHashMap::default(),
        }
    }
}
