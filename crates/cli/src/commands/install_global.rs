use crate::helpers::create_progress_bar;
use proto_core::{load_tool, Id};
use starbase::SystemResult;
use starbase_styles::color;
use std::{env, process};
use tracing::{debug, info};

pub async fn install_global(tool_id: Id, dependencies: Vec<String>) -> SystemResult {
    let mut tool = load_tool(&tool_id).await?;
    tool.locate_globals_dir().await?;

    let globals_dir = tool.get_globals_bin_dir();
    let mut log_list = vec![];

    if !tool.plugin.has_func("install_global") || globals_dir.is_none() {
        eprintln!(
            "{} does not support installing global dependencies",
            tool.get_name()
        );
        process::exit(1);
    }

    for dependency in dependencies {
        env::set_var(
            "PROTO_INSTALL_GLOBAL",
            format!("{}:{}", tool_id, dependency),
        );

        log_list.push(color::id(&dependency));

        debug!(
            tool = tool.id.as_str(),
            dependency, "Installing global dependency"
        );

        let pb = create_progress_bar(format!("Installing {} for {}", dependency, tool.get_name()));

        tool.install_global(&dependency).await?;

        pb.finish_and_clear();
    }

    info!(
        "Installed {} to {}!",
        log_list.join(", "),
        color::path(globals_dir.as_ref().unwrap()),
    );

    Ok(())
}
