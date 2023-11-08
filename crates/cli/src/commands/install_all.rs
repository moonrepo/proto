use crate::helpers::{
    create_progress_bar, disable_progress_bars, enable_progress_bars, ToolsLoader,
};
use crate::{
    commands::clean::{internal_clean, CleanArgs},
    commands::install::{internal_install, InstallArgs},
};
use miette::IntoDiagnostic;
use starbase::system;
use starbase_styles::color;
use std::{env, process};
use tracing::{debug, info};

#[system]
pub async fn install_all() {
    let working_dir = env::current_dir().expect("Missing current directory.");

    debug!("Loading tools and plugins from .prototools");

    let loader = ToolsLoader::new()?;
    let tools = loader.load_tools().await?;

    debug!("Detecting tool versions to install");

    let mut versions = loader.tools_config.tools.clone();

    for tool in &tools {
        if versions.contains_key(&tool.id) {
            continue;
        }

        if let Some(candidate) = tool.detect_version_from(&working_dir).await? {
            debug!("Detected version {} for {}", candidate, tool.get_name());

            versions.insert(tool.id.clone(), candidate);
        }
    }

    if versions.is_empty() {
        eprintln!("Nothing to install!");
        process::exit(1);
    }

    let pb = create_progress_bar(format!(
        "Installing {} tools: {}",
        versions.len(),
        versions
            .keys()
            .map(color::id)
            .collect::<Vec<_>>()
            .join(", ")
    ));

    disable_progress_bars();

    let mut futures = vec![];

    for tool in tools {
        if let Some(version) = versions.remove(&tool.id) {
            futures.push(tokio::spawn(async {
                internal_install(
                    InstallArgs {
                        canary: false,
                        id: tool.id.clone(),
                        pin: false,
                        passthrough: vec![],
                        spec: Some(version),
                    },
                    Some(tool),
                )
                .await
            }));
        }
    }

    for future in futures {
        future.await.into_diagnostic()??;
    }

    enable_progress_bars();

    pb.finish_and_clear();

    info!("Successfully installed tools");

    if loader.user_config.auto_clean {
        info!("Auto-clean enabled, starting clean");

        internal_clean(&CleanArgs {
            yes: true,
            ..Default::default()
        })
        .await?;
    }
}
