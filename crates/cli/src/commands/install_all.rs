use crate::helpers::{create_progress_bar, disable_progress_bars, enable_progress_bars};
use crate::session::ProtoSession;
use crate::{
    commands::clean::{internal_clean, CleanArgs},
    commands::install::{internal_install, InstallArgs},
};
use miette::IntoDiagnostic;
use starbase::AppResult;
use starbase_styles::color;
use std::process;
use tracing::debug;

pub async fn install_all(session: ProtoSession) -> AppResult {
    debug!("Loading tools and plugins from .prototools");

    let tools = session.load_tools().await?;

    debug!("Detecting tool versions to install");

    let config = session
        .env
        .load_config_manager()?
        .get_merged_config_without_global()?;
    let mut versions = config.versions.to_owned();

    for tool in &tools {
        if versions.contains_key(&tool.id) {
            continue;
        }

        if let Some((candidate, _)) = tool.detect_version_from(&session.env.cwd).await? {
            debug!("Detected version {} for {}", candidate, tool.get_name());

            versions.insert(tool.id.clone(), candidate);
        }
    }

    if versions.is_empty() {
        eprintln!("No versions have been configured, nothing to install!");

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

    // Then install each tool in parallel!
    let mut futures = vec![];

    for tool in tools {
        if let Some(version) = versions.remove(&tool.id) {
            let proto_clone = session.clone();

            futures.push(tokio::spawn(async move {
                internal_install(
                    &proto_clone,
                    InstallArgs {
                        canary: false,
                        id: tool.id.clone(),
                        pin: None,
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

    println!("Successfully installed tools!");

    if config.settings.auto_clean {
        debug!("Auto-clean enabled, starting clean");

        internal_clean(&session, &CleanArgs::default(), true).await?;
    }

    Ok(())
}
