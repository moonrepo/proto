use crate::helpers::{create_progress_bar, enable_logging};
use crate::tools::{create_tool, ToolType};
use starbase::SystemResult;
use tracing::{debug, info};

pub async fn uninstall(tool_type: ToolType, version: String) -> SystemResult {
    enable_logging();

    let mut tool = create_tool(&tool_type)?;

    if !tool.is_setup(&version).await? {
        info!(
            "{} v{} does not exist!",
            tool.get_name(),
            tool.get_resolved_version(),
        );

        return Ok(());
    }

    debug!(
        "Uninstalling {} with version \"{}\"",
        tool.get_name(),
        version,
    );

    let pb = create_progress_bar(format!(
        "Uninstalling {} v{}",
        tool.get_name(),
        tool.get_resolved_version()
    ));

    tool.teardown().await?;

    pb.finish_and_clear();

    info!(
        "{} v{} has been uninstalled!",
        tool.get_name(),
        tool.get_resolved_version(),
    );

    Ok(())
}
