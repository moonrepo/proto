use crate::commands::clean::purge_tool;
use crate::helpers::{create_progress_bar, disable_progress_bars, ProtoResource};
use clap::Args;
use proto_core::{Id, UnresolvedVersionSpec};
use starbase::system;
use tracing::{debug, info};

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
        purge_tool(proto, &args.id, args.yes).await?;

        return Ok(());
    };

    // Uninstall a tool by version
    let mut tool = proto.load_tool(&args.id).await?;

    if !tool.is_setup(spec).await? {
        info!(
            "{} {} does not exist!",
            tool.get_name(),
            tool.get_resolved_version(),
        );

        return Ok(());
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

    info!(
        "{} {} has been uninstalled!",
        tool.get_name(),
        tool.get_resolved_version(),
    );
}
