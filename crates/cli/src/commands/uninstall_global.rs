use crate::helpers::create_progress_bar;
use clap::Args;
use proto_core::{load_tool, Id};
use starbase::system;
use starbase_styles::color;
use std::process;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct UninstallGlobalArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Dependencies to uninstall")]
    dependencies: Vec<String>,
}

#[system]
pub async fn uninstall_global(args: ArgsRef<UninstallGlobalArgs>) {
    let mut tool = load_tool(&args.id).await?;
    tool.locate_globals_dir().await?;

    let globals_dir = tool.get_globals_bin_dir();
    let mut log_list = vec![];

    if !tool.plugin.has_func("uninstall_global") || globals_dir.is_none() {
        eprintln!(
            "{} does not support uninstalling global dependencies",
            tool.get_name()
        );
        process::exit(1);
    }

    for dependency in &args.dependencies {
        log_list.push(color::id(&dependency));

        debug!(
            tool = tool.id.as_str(),
            dependency, "Uninstalling global dependency"
        );

        let pb = create_progress_bar(format!(
            "Uninstalling {} for {}",
            dependency,
            tool.get_name()
        ));

        tool.uninstall_global(&dependency).await?;

        pb.finish_and_clear();
    }

    info!(
        "Uninstalled {} from {}!",
        log_list.join(", "),
        color::path(globals_dir.as_ref().unwrap()),
    );
}
