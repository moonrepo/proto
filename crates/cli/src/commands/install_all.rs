use crate::helpers::disable_progress_bars;
use crate::tools::ToolType;
use crate::{commands::clean::clean, commands::install::install, helpers::create_progress_bar};
use proto::UserConfig;
use proto_core::{ProtoError, ToolsConfig, TOOLS_CONFIG_NAME};
use starbase::SystemResult;
use std::{env, str::FromStr};
use tracing::info;

pub async fn install_all() -> SystemResult {
    let current_dir = env::current_dir().expect("Invalid working directory!");

    let Some(config) = ToolsConfig::load_upwards(&current_dir)? else {
        return Err(ProtoError::MissingConfig(TOOLS_CONFIG_NAME.to_owned()))?;
    };

    let mut futures = vec![];
    let pb = create_progress_bar(format!(
        "Installing {} tools: {}",
        config.tools.len(),
        config.tools.keys().cloned().collect::<Vec<_>>().join(", ")
    ));

    // Don't show inner progress bars
    disable_progress_bars();

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

    if UserConfig::load()?.auto_clean {
        info!("Auto-clean enabled");
        clean(None, true).await?;
    }

    Ok(())
}
