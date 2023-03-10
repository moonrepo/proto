use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use log::info;
use proto_core::ProtoError;

pub async fn uninstall(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let mut tool = create_tool(&tool_type)?;

    if !tool.is_setup(&version).await? {
        info!(
            target: "proto:uninstall",
            "{} v{} does not exist!",
            tool.get_name(),
            tool.get_resolved_version(),
        );

        return Ok(());
    }

    info!(
        target: "proto:uninstall",
        "Uninstalling {} with version \"{}\"",
        tool.get_name(),
        version,
    );

    tool.teardown().await?;

    info!(
        target: "proto:uninstall",
        "{} v{} has been uninstalled!",
        tool.get_name(),
        tool.get_resolved_version(),
    );

    Ok(())
}
