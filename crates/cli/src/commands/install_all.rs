use crate::helpers::disable_progress_bars;
use crate::states::ToolsConfig;
use crate::tools::ToolType;
use crate::{commands::clean::clean, commands::install::install, helpers::create_progress_bar};
use proto::UserConfig;
use proto_core::{ProtoError, TOOLS_CONFIG_NAME};
use starbase::SystemResult;
use std::str::FromStr;
use tracing::info;

pub async fn install_all(tools_config: &ToolsConfig) -> SystemResult {
    let Some(config) = &tools_config.0 else {
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

    for (tool, version) in &config.tools {
        futures.push(install(
            ToolType::from_str(tool)?,
            Some(version.to_owned()),
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
