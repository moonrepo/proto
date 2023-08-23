use proto_core::{load_tool, Id};
use starbase::SystemResult;
use starbase_styles::color;
use tracing::info;

pub async fn unalias(tool_id: Id, alias: String) -> SystemResult {
    let mut tool = load_tool(&tool_id).await?;

    let value = tool.manifest.aliases.remove(&alias);
    tool.manifest.save()?;

    if let Some(version) = value {
        info!(
            "Removed alias {} ({}) from {}",
            color::id(alias),
            color::muted_light(version.to_string()),
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
