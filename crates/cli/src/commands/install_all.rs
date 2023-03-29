use crate::helpers::enable_logging;
use crate::tools::ToolType;
use crate::{commands::install::install, helpers::create_progress_bar};
use proto_core::{ProtoError, ToolsConfig, TOOLS_CONFIG_NAME};
use std::{env, str::FromStr};

pub async fn install_all() -> Result<(), ProtoError> {
    enable_logging();

    let current_dir = env::current_dir().expect("Invalid working directory!");

    let Some(config) = ToolsConfig::load_upwards(&current_dir)? else {
        return Err(ProtoError::MissingConfig(TOOLS_CONFIG_NAME.to_owned()));
    };

    let mut futures = vec![];
    let pb = create_progress_bar(format!(
        "Installing {} tools: {}",
        config.tools.len(),
        config.tools.keys().cloned().collect::<Vec<_>>().join(", ")
    ));

    // Don't show inner progress bars
    env::set_var("PROTO_NO_PROGRESS", "1");

    for (tool, version) in config.tools {
        futures.push(install(
            ToolType::from_str(&tool)?,
            Some(version),
            false,
            vec![],
        ));
    }

    futures::future::try_join_all(futures).await?;

    pb.finish_and_clear();

    Ok(())
}
