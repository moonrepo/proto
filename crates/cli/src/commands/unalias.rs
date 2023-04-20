use crate::tools::{create_tool, ToolType};
use proto_core::{color, Manifest};
use starbase::SystemResult;
use tracing::info;

pub async fn unalias(tool_type: ToolType, alias: String) -> SystemResult {
    let tool = create_tool(&tool_type).await?;

    let mut manifest = Manifest::load(tool.get_manifest_path())?;
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
