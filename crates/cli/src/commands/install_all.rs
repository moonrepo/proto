use crate::helpers::{disable_progress_bars, enable_progress_bars};
use crate::tools::{create_plugin_from_locator, create_tool, ToolType};
use crate::{commands::clean::clean, commands::install::install, helpers::create_progress_bar};
use futures::future::try_join_all;
use proto_core::{expand_detected_version, Proto, ToolsConfig, UserConfig};
use starbase::SystemResult;
use std::env;
use std::str::FromStr;
use strum::IntoEnumIterator;
use tracing::{debug, info};

pub async fn install_all() -> SystemResult {
    let working_dir = env::current_dir().expect("Missing current directory.");

    // Inherit from .prototools
    debug!("Detecting tools and plugins from .prototools");

    let mut config = ToolsConfig::load_upwards()?;

    // Detect from working dir
    debug!("Detecting tools from environment");

    for tool_type in ToolType::iter() {
        if let ToolType::Plugin(_) = tool_type {
            continue;
        }

        let tool = create_tool(&tool_type).await?;

        if let Some(version) = tool.detect_version_from(&working_dir).await? {
            if let Some(version) = expand_detected_version(&version, tool.get_manifest()?)? {
                debug!(version, "Detected version for {}", tool.get_name());

                config.tools.insert(tool.get_id().to_owned(), version);
            }
        }
    }

    let tools = config.tools;
    let plugins = config.plugins;

    if !plugins.is_empty() {
        let proto = Proto::new()?;
        let mut futures = vec![];
        let pb = create_progress_bar(format!(
            "Installing {} plugins: {}",
            plugins.len(),
            plugins.keys().cloned().collect::<Vec<_>>().join(", ")
        ));

        for (name, locator) in &plugins {
            futures.push(create_plugin_from_locator(name, &proto, locator));
        }

        try_join_all(futures).await?;

        pb.finish_and_clear();
    }

    if !tools.is_empty() {
        let mut futures = vec![];
        let pb = create_progress_bar(format!(
            "Installing {} tools: {}",
            tools.len(),
            tools.keys().cloned().collect::<Vec<_>>().join(", ")
        ));

        disable_progress_bars();

        for (tool, version) in &tools {
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

    if tools.is_empty() && plugins.is_empty() {
        info!("Nothing to install!")
    } else {
        info!("Successfully installed tools and plugins");
    }

    if UserConfig::load()?.auto_clean {
        debug!("Auto-clean enabled, starting clean");
        clean(None, true).await?;
    }

    Ok(())
}
