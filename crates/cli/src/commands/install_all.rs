use crate::helpers::{disable_progress_bars, enable_progress_bars};
use crate::tools::create_tool_from_plugin;
use crate::{commands::clean::clean, commands::install::install, helpers::create_progress_bar};
use futures::future::try_join_all;
use proto_core::{expand_detected_version, ProtoEnvironment, ToolsConfig, UserConfig};
use starbase::SystemResult;
use starbase_styles::color;
use std::env;
use tracing::{debug, info};

pub async fn install_all() -> SystemResult {
    let working_dir = env::current_dir().expect("Missing current directory.");

    debug!("Loading tools and plugins from .prototools");

    let mut config = ToolsConfig::load_upwards()?;
    config.inherit_builtin_plugins();

    debug!("Detecting tool versions to install");

    let proto = ProtoEnvironment::new()?;

    for (name, locator) in config.plugins {
        if config.tools.contains_key(&name) {
            continue;
        }

        let tool = create_tool_from_plugin(&name, &proto, &locator).await?;

        if let Some(candidate) = tool.detect_version_from(&working_dir).await? {
            if let Some(version) = expand_detected_version(&candidate, &tool.manifest)? {
                debug!("Detected version {} for {}", version, tool.get_name());

                config.tools.insert(name, version);
            }
        }
    }

    let tools = config.tools;

    if tools.is_empty() {
        info!("Nothing to install!");
    } else {
        let mut futures = vec![];
        let pb = create_progress_bar(format!(
            "Installing {} tools: {}",
            tools.len(),
            tools
                .keys()
                .map(|k| color::id(k))
                .collect::<Vec<_>>()
                .join(", ")
        ));

        disable_progress_bars();

        for (id, version) in tools {
            futures.push(install(id, Some(version.to_string()), false, vec![]));
        }

        try_join_all(futures).await?;

        enable_progress_bars();

        pb.finish_and_clear();

        info!("Successfully installed tools");
    }

    if UserConfig::load()?.auto_clean {
        debug!("Auto-clean enabled, starting clean");
        clean(None, true).await?;
    }

    Ok(())
}
