use crate::helpers::{disable_progress_bars, enable_progress_bars};
use crate::states::{PluginList, ToolsConfig, UserConfig};
use crate::tools::{create_plugin_from_locator, create_tool, ToolType};
use crate::{commands::clean::clean, commands::install::install, helpers::create_progress_bar};
use futures::future::try_join_all;
use proto_core::{expand_detected_version, Proto};
use rustc_hash::FxHashMap;
use starbase::SystemResult;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use strum::IntoEnumIterator;
use tracing::{debug, info};

pub async fn install_all(
    tools_config: &ToolsConfig,
    user_config: &UserConfig,
    plugin_list: &PluginList,
) -> SystemResult {
    let mut tools = FxHashMap::default();
    let mut plugins = FxHashMap::default();
    let mut config_dir = PathBuf::new();
    let working_dir = env::current_dir().expect("Missing current directory.");

    // Inherit from .prototools
    if let Some(config) = &tools_config.0 {
        debug!(config = %config.path.display(), "Detecting tools and plugins from .prototools");

        if !config.tools.is_empty() {
            tools.extend(config.tools.clone());
        }

        if !config.plugins.is_empty() {
            plugins.extend(config.plugins.clone());
            config_dir = config.path.parent().unwrap().to_path_buf();
        }
    }

    // Detect from working dir
    debug!("Detecting tools from environment");

    for tool_type in ToolType::iter() {
        if matches!(tool_type, ToolType::Plugin(_)) {
            continue;
        }

        let tool = create_tool(&tool_type).await?;

        if let Some(version) = tool.detect_version_from(&working_dir).await? {
            if let Some(version) = expand_detected_version(&version, tool.get_manifest()?)? {
                debug!(version, "Detected version for {}", tool.get_name());

                tools.insert(tool.get_bin_name().to_owned(), version);
            }
        }
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

    if !plugins.is_empty() {
        let proto = Proto::new()?;
        let mut futures = vec![];
        let pb = create_progress_bar(format!(
            "Installing {} plugins: {}",
            plugins.len(),
            plugins.keys().cloned().collect::<Vec<_>>().join(", ")
        ));

        for (name, locator) in &plugins {
            futures.push(create_plugin_from_locator(
                name,
                &proto,
                locator,
                &config_dir,
            ));
        }

        try_join_all(futures).await?;

        pb.finish_and_clear();
    }

    if tools.is_empty() && plugins.is_empty() {
        info!("Nothing to install!")
    } else {
        info!("Successfully installed tools and plugins");
    }

    if user_config.0.auto_clean {
        debug!("Auto-clean enabled, starting clean");
        clean(None, true, plugin_list).await?;
    }

    Ok(())
}
