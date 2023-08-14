use crate::helpers::create_progress_bar;
use crate::tools::create_tool;
use proto_core::{Id, ProtoError};
use proto_pdk_api::{UninstallGlobalInput, UninstallGlobalOutput};
use starbase::SystemResult;
use starbase_styles::color;
use std::process;
use tracing::{debug, info};

pub async fn uninstall_global(tool_id: Id, dependencies: Vec<String>) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;
    tool.locate_globals_dir().await?;

    let Some(globals_dir) = tool.get_globals_bin_dir() else {
        eprintln!("{} does not support global dependencies", tool.get_name());
        process::exit(1);
    };

    let mut log_list = vec![];

    for dependency in dependencies {
        debug!(
            tool = tool.id.as_str(),
            dependency, "Uninstalling global dependency"
        );

        let pb = create_progress_bar(format!(
            "Uninstalling {} for {}",
            dependency,
            tool.get_name()
        ));

        log_list.push(color::id(&dependency));

        let result: UninstallGlobalOutput = tool.plugin.call_func_with(
            "uninstall_global",
            UninstallGlobalInput {
                env: tool.create_environment()?,
                dependency,
                globals_dir: tool.plugin.to_virtual_path(globals_dir),
            },
        )?;

        pb.finish_and_clear();

        if !result.uninstalled {
            return Err(ProtoError::Message(
                result.error.unwrap_or("Unknown failure!".into()),
            ))?;
        }
    }

    info!(
        "Uninstalled {} from {}!",
        log_list.join(", "),
        color::path(globals_dir),
    );

    Ok(())
}
