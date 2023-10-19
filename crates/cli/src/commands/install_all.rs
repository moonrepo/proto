use crate::helpers::{disable_progress_bars, enable_progress_bars};
use crate::{
    commands::clean::{internal_clean, CleanArgs},
    commands::install::{internal_install, InstallArgs},
    helpers::create_progress_bar,
};
use futures::future::try_join_all;
use miette::IntoDiagnostic;
use proto_core::{load_tool_from_locator, ProtoEnvironment, ToolsConfig, UserConfig};
use starbase::system;
use starbase_styles::color;
use std::env;
use tracing::{debug, info};

#[system]
pub async fn install_all() {
    let working_dir = env::current_dir().expect("Missing current directory.");

    debug!("Loading tools and plugins from .prototools");

    let user_config = UserConfig::load()?;
    let mut config = ToolsConfig::load_upwards()?;
    config.inherit_builtin_plugins();

    debug!("Detecting tool versions to install");

    let proto = ProtoEnvironment::new()?;

    for (name, locator) in config.plugins {
        if config.tools.contains_key(&name) {
            continue;
        }

        let tool = load_tool_from_locator(&name, &proto, &locator, &user_config).await?;

        if let Some(candidate) = tool.detect_version_from(&working_dir).await? {
            debug!("Detected version {} for {}", candidate, tool.get_name());

            config.tools.insert(name, candidate);
        }
    }

    if config.tools.is_empty() {
        info!("Nothing to install!");
    } else {
        let pb = create_progress_bar(format!(
            "Installing {} tools: {}",
            config.tools.len(),
            config
                .tools
                .keys()
                .map(color::id)
                .collect::<Vec<_>>()
                .join(", ")
        ));

        disable_progress_bars();

        let mut futures = vec![];

        for (id, version) in config.tools {
            futures.push(tokio::spawn(async {
                internal_install(InstallArgs {
                    canary: false,
                    id,
                    pin: false,
                    passthrough: vec![],
                    spec: Some(version),
                })
                .await
            }));
        }

        try_join_all(futures).await.into_diagnostic()?;

        enable_progress_bars();

        pb.finish_and_clear();

        info!("Successfully installed tools");
    }

    if user_config.auto_clean {
        debug!("Auto-clean enabled, starting clean");

        internal_clean(&CleanArgs {
            yes: true,
            ..Default::default()
        })
        .await?;
    }
}
