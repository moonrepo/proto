use crate::{errors::ProtoError, get_root};
use serde::Deserialize;
use std::fs;

pub const CONFIG_NAME: &str = "config.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    pub auto_install: bool,
}

impl UserConfig {
    pub fn load() -> Result<Self, ProtoError> {
        let path = get_root()?.join(CONFIG_NAME);

        if !path.exists() {
            return Ok(UserConfig::default());
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?;

        let config: UserConfig = toml::from_str(&contents)
            .map_err(|e| ProtoError::Toml(path.to_path_buf(), e.to_string()))?;

        Ok(config)
    }
}
