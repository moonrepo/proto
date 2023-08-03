use crate::helpers::create_progress_bar;
use crate::tools::create_tool;
use starbase::SystemResult;
use tracing::{debug, info};

pub async fn uninstall(tool_id: String, version: String) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;

    if !tool.is_setup(&version).await? {
        info!(
            "{} {} does not exist!",
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

    Ok(())
}
