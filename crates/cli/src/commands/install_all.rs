use crate::helpers::{disable_progress_bars, enable_progress_bars};
use crate::states::ToolsConfig;
use crate::tools::ToolType;
use crate::{commands::clean::clean, commands::install::install, helpers::create_progress_bar};
use futures::future::try_join_all;
use proto::{create_plugin_from_locator, Proto, UserConfig};
use proto_core::{ProtoError, TOOLS_CONFIG_NAME};
use starbase::SystemResult;
use std::str::FromStr;
use tracing::info;

pub async fn install_all(tools_config: &ToolsConfig) -> SystemResult {
    let Some(config) = &tools_config.0 else {
        return Err(ProtoError::MissingConfig(TOOLS_CONFIG_NAME.to_owned()))?;
    };

    if !config.tools.is_empty() {
        let mut futures = vec![];
        let pb = create_progress_bar(format!(
            "Installing {} tools: {}",
            config.tools.len(),
            config.tools.keys().cloned().collect::<Vec<_>>().join(", ")
        ));

        disable_progress_bars();

        for (tool, version) in &config.tools {
            futures.push(install(
                ToolType::from_str(tool)?,
                Some(version.to_owned()),
                false,
                vec![],
            ));
        }

        try_join_all(futures).await?;
        enable_progress_bars();

        pb.finish_and_clear();
    }

    if !config.plugins.is_empty() {
        let mut futures = vec![];
        let pb = create_progress_bar(format!(
            "Installing {} plugins: {}",
            config.plugins.len(),
            config
                .plugins
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));

        for (name, locator) in &config.plugins {
            futures.push(create_plugin_from_locator(
                name,
                Proto::new()?,
                locator,
                config.path.parent().unwrap(),
            ));
        }

        try_join_all(futures).await?;

        pb.finish_and_clear();
    }

    if UserConfig::load()?.auto_clean {
        info!("Auto-clean enabled");
        clean(None, true).await?;
    }

    Ok(())
}
