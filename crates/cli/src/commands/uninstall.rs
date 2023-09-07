use crate::helpers::{create_progress_bar, disable_progress_bars};
use clap::Args;
use proto_core::{load_tool, Id, UnresolvedVersionSpec};
use starbase::system;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct UninstallArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    semver: UnresolvedVersionSpec,
}

#[system]
pub async fn uninstall(args: ArgsRef<UninstallArgs>) {
    let mut tool = load_tool(&args.id).await?;

    if !tool.is_setup(&args.semver).await? {
        info!(
            "{} {} does not exist!",
            tool.get_name(),
            tool.get_resolved_version(),
        );

        return Ok(());
    }

    debug!(
        "Uninstalling {} with version {}",
        tool.get_name(),
        args.semver
    );

    if tool.disable_progress_bars() {
        disable_progress_bars();
    }

    let pb = create_progress_bar(format!(
        "Uninstalling {} {}",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    tool.teardown().await?;

    pb.finish_and_clear();

    info!(
        "{} {} has been uninstalled!",
        tool.get_name(),
        tool.get_resolved_version(),
    );
}
