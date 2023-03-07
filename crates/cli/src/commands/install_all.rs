use std::env;

use crate::commands::install::install;
use crate::config::{Config, CONFIG_NAME};
use proto::ProtoError;

pub async fn install_all() -> Result<(), ProtoError> {
    let current_dir = env::current_dir().expect("Invalid working directory!");

    let Some(config) = Config::find_upwards(&current_dir)? else {
        return Err(ProtoError::MissingConfig(CONFIG_NAME.to_owned()));
    };

    let mut futures = vec![];

    for (tool, version) in config.tools {
        futures.push(install(tool, Some(version)));
    }

    for future in futures {
        future.await?;
    }

    Ok(())
}
