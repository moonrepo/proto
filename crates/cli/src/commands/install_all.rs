use crate::helpers::{
    create_progress_bar, disable_progress_bars, enable_progress_bars, ProtoResource,
};
use crate::{
    commands::clean::{internal_clean, CleanArgs},
    commands::install::{internal_install, InstallArgs},
};
use miette::IntoDiagnostic;
use starbase::system;
use starbase_styles::color;
use std::process;
use tracing::{debug, info};

#[system]
pub async fn install_all(proto: ResourceRef<ProtoResource>) {
    debug!("Loading tools and plugins from .prototools");

    let tools = proto.load_tools().await?;

    debug!("Detecting tool versions to install");

    let config = proto
        .env
        .load_config_manager()?
        .get_merged_config_without_global()?;
    let mut versions = config.versions.to_owned();

    for tool in &tools {
        if versions.contains_key(&tool.id) {
            continue;
        }

        if let Some(candidate) = tool.detect_version_from(&proto.env.cwd).await? {
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

    // Then install each tool in parallel!
    let mut futures = vec![];

    for tool in tools {
        if let Some(version) = versions.remove(&tool.id) {
            let proto_clone = proto.clone();

            futures.push(tokio::spawn(async move {
                internal_install(
                    &proto_clone,
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

    if config.settings.auto_clean {
        info!("Auto-clean enabled, starting clean");

        internal_clean(
            proto,
            &CleanArgs {
                yes: true,
                ..Default::default()
            },
        )
        .await?;
    }
}
