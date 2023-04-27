use crate::tools::{create_tool, ToolType};
use proto_core::color;
use starbase::SystemResult;
use tracing::info;

pub async fn unalias(tool_type: ToolType, alias: String) -> SystemResult {
    let mut tool = create_tool(&tool_type).await?;

    let manifest = tool.get_manifest_mut()?;
    let value = manifest.aliases.remove(&alias);
    manifest.save()?;

    if let Some(version) = value {
        info!(
            "Removed alias {} ({}) from {}",
            color::id(alias),
            color::muted_light(version),
            tool.get_name(),
        );
    } else {
        info!(
            "Alias {} not found for {}",
            color::id(alias),
            tool.get_name(),
        );
    }

    Ok(())
}
