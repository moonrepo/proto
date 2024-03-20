use crate::commands::clean::purge_tool;
use crate::helpers::{create_progress_bar, disable_progress_bars, ProtoResource};
use crate::telemetry::{track_usage, Metric};
use clap::Args;
use proto_core::{Id, Tool, UnresolvedVersionSpec};
use starbase::system;
use std::process;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct UninstallArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(help = "Version or alias of tool")]
    semver: Option<UnresolvedVersionSpec>,

    #[arg(long, help = "Avoid and force confirm prompts")]
    yes: bool,
}

#[system]
pub async fn uninstall(args: ArgsRef<UninstallArgs>, proto: ResourceRef<ProtoResource>) {
    // Uninstall everything
    let Some(spec) = &args.semver else {
        let tool = purge_tool(proto, &args.id, args.yes).await?;

        // Track usage metrics
        track_uninstall(&tool, true).await?;

        return Ok(());
    };

    // Uninstall a tool by version
    let mut tool = proto.load_tool(&args.id).await?;

    if !tool.is_setup(spec).await? {
        eprintln!(
            "{} {} has not been installed locally",
            tool.get_name(),
            tool.get_resolved_version(),
        );

        process::exit(1);
    }

    debug!("Uninstalling {} with version {}", tool.get_name(), spec);

    if tool.disable_progress_bars() {
        disable_progress_bars();
    }

    let pb = create_progress_bar(format!(
        "Uninstalling {} {}",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    let uninstalled = tool.teardown().await?;

    pb.finish_and_clear();

    if !uninstalled {
        return Ok(());
    }

    // Track usage metrics
    track_uninstall(&tool, false).await?;

    println!(
        "{} {} has been uninstalled!",
        tool.get_name(),
        tool.get_resolved_version(),
    );
}

async fn track_uninstall(tool: &Tool, purged: bool) -> miette::Result<()> {
    track_usage(
        &tool.proto,
        Metric::UninstallTool {
            id: tool.id.to_string(),
            plugin: tool
                .locator
                .as_ref()
                .map(|loc| loc.to_string())
                .unwrap_or_default(),
            version: if purged {
                "*".into()
            } else {
                tool.get_resolved_version().to_string()
            },
        },
    )
    .await
}
