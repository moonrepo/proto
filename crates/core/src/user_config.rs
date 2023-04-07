use crate::{errors::ProtoError, helpers::get_root};
use serde::Deserialize;
use starbase_utils::fs;

pub const USER_CONFIG_NAME: &str = "config.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserConfig {
    pub auto_install: bool,
}

impl UserConfig {
    pub fn load() -> Result<Self, ProtoError> {
        let path = get_root()?.join(USER_CONFIG_NAME);

        if !path.exists() {
            return Ok(UserConfig::default());
        }

        let contents = fs::read(&path)?;

        let config: UserConfig = toml::from_str(&contents).map_err(|error| ProtoError::Toml {
            path: path.to_path_buf(),
            error,
        })?;

        Ok(config)
    }
}
