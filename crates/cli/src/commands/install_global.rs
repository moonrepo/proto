use crate::error::ProtoCliError;
use crate::helpers::create_progress_bar;
use clap::Args;
use proto_core::{load_tool, Id};
use starbase::system;
use starbase_styles::color;
use std::env;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct InstallGlobalArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Dependencies and optional version to install")]
    dependencies: Vec<String>,
}

#[system]
pub async fn install_global(args: ArgsRef<InstallGlobalArgs>) {
    let mut tool = load_tool(&args.id).await?;
    tool.locate_globals_dir().await?;

    let globals_dir = tool.get_globals_bin_dir();
    let mut log_list = vec![];

    if !tool.plugin.has_func("install_global") || globals_dir.is_none() {
        return Err(ProtoCliError::GlobalsNotSupported {
            tool: tool.get_name().to_owned(),
        }
        .into());
    }

    for dependency in &args.dependencies {
        env::set_var(
            "PROTO_INSTALL_GLOBAL",
            format!("{}:{}", args.id, dependency),
        );

        log_list.push(color::id(dependency));

        debug!(
            tool = tool.id.as_str(),
            dependency, "Installing global dependency"
        );

        let pb = create_progress_bar(format!("Installing {} for {}", dependency, tool.get_name()));

        tool.install_global(dependency).await?;

        pb.finish_and_clear();
    }

    info!(
        "Installed {} to {}!",
        log_list.join(", "),
        color::path(globals_dir.as_ref().unwrap()),
    );
}
