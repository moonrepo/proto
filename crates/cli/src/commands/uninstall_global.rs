use crate::helpers::create_progress_bar;
use proto_core::{load_tool, Id};
use starbase::SystemResult;
use starbase_styles::color;
use std::process;
use tracing::{debug, info};

pub async fn uninstall_global(tool_id: Id, dependencies: Vec<String>) -> SystemResult {
    let mut tool = load_tool(&tool_id).await?;
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

    for dependency in dependencies {
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

    Ok(())
}
