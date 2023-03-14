use std::env;

use crate::commands::install::install;
use crate::config::{Config, CONFIG_NAME};
use crate::helpers::enable_logging;
use proto_core::ProtoError;

pub async fn install_all() -> Result<(), ProtoError> {
    enable_logging();

    let current_dir = env::current_dir().expect("Invalid working directory!");

    let Some(config) = Config::load_upwards(&current_dir)? else {
        return Err(ProtoError::MissingConfig(CONFIG_NAME.to_owned()));
    };

    let mut futures = vec![];

    for (tool, version) in config.tools {
        futures.push(install(tool, Some(version), false, vec![]));
    }

    futures::future::try_join_all(futures).await?;

    Ok(())
}
