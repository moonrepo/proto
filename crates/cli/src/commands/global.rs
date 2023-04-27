use crate::tools::{create_tool, ToolType};
use proto_core::color;
use starbase::SystemResult;
use tracing::{debug, info};

pub async fn global(tool_type: ToolType, version: String) -> SystemResult {
    let mut tool = create_tool(&tool_type).await?;

    let manifest = tool.get_manifest_mut()?;
    manifest.default_version = Some(version.clone());
    manifest.save()?;

    debug!(
        "Wrote the global version to {}",
        color::path(&manifest.path),
    );

    info!("Set the global {} version to {}", tool.get_name(), version);

    Ok(())
}
