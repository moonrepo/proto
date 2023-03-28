use crate::commands::install::install;
use crate::helpers::enable_logging;
use crate::tools::ToolType;
use proto_core::{ProtoError, ToolsConfig, TOOLS_CONFIG_NAME};
use std::{env, str::FromStr};

pub async fn install_all() -> Result<(), ProtoError> {
    enable_logging();

    let current_dir = env::current_dir().expect("Invalid working directory!");

    let Some(config) = ToolsConfig::load_upwards(&current_dir)? else {
        return Err(ProtoError::MissingConfig(TOOLS_CONFIG_NAME.to_owned()));
    };

    let mut futures = vec![];

    for (tool, version) in config.tools {
        futures.push(install(
            ToolType::from_str(&tool)?,
            Some(version),
            false,
            vec![],
        ));
    }

    futures::future::try_join_all(futures).await?;

    Ok(())
}
