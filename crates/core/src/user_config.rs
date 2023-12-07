#![allow(deprecated)]

use crate::proto_config::{DetectStrategy, PinType};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, toml};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::debug;
use warpgate::{HttpOptions, Id, PluginLocator};

pub const USER_CONFIG_NAME: &str = "config.toml";

#[deprecated]
#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserConfig {
    pub auto_clean: Option<bool>,
    pub auto_install: Option<bool>,
    pub detect_strategy: Option<DetectStrategy>,
    pub node_intercept_globals: Option<bool>,
    pub pin_latest: Option<PinType>,
    pub http: Option<HttpOptions>,
    pub plugins: BTreeMap<Id, PluginLocator>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl UserConfig {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        let dir = dir.as_ref();
        let path = dir.join(USER_CONFIG_NAME);

        if !path.exists() {
            return Ok(UserConfig {
                path,
                ..UserConfig::default()
            });
        }

        debug!(file = ?path, "Loading {}", USER_CONFIG_NAME);

        let contents = fs::read_file_with_lock(&path)?;
        let mut config: UserConfig = toml::from_str(&contents).into_diagnostic()?;

        config.path = path;

        Ok(config)
    }
}
