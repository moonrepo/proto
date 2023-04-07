use crate::{errors::ProtoError, helpers::get_root};
use serde::Deserialize;
use starbase_utils::toml;

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

        let config: UserConfig = toml::read_file(&path)?;

        Ok(config)
    }
}
