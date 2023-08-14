use crate::helpers::create_progress_bar;
use crate::tools::create_tool;
use proto_core::{Id, ProtoError};
use proto_pdk_api::{InstallGlobalInput, InstallGlobalOutput};
use starbase::SystemResult;
use starbase_styles::color;
use std::process;
use tracing::{debug, info};

pub async fn install_global(tool_id: Id, dependencies: Vec<String>) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;
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
        log_list.push(color::id(&dependency));

        debug!(
            tool = tool.id.as_str(),
            dependency, "Installing global dependency"
        );

        let pb = create_progress_bar(format!("Installing {} for {}", dependency, tool.get_name()));

        let result: InstallGlobalOutput = tool.plugin.call_func_with(
            "install_global",
            InstallGlobalInput {
                env: tool.create_environment()?,
                dependency,
                globals_dir: tool.plugin.to_virtual_path(globals_dir.as_ref().unwrap()),
            },
        )?;

        pb.finish_and_clear();

        if !result.installed {
            return Err(ProtoError::Message(
                result.error.unwrap_or("Unknown failure!".into()),
            ))?;
        }
    }

    info!(
        "Installed {} to {}!",
        log_list.join(", "),
        color::path(globals_dir.as_ref().unwrap()),
    );

    Ok(())
}
