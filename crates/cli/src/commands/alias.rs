use crate::tools::{create_tool, ToolType};
use proto_core::{color, is_alias_name, ProtoError};
use starbase::SystemResult;
use tracing::info;

pub async fn alias(tool_type: ToolType, alias: String, version: String) -> SystemResult {
    if alias == version {
        return Err(ProtoError::Message("Cannot map an alias to itself.".into()))?;
    }

    if !is_alias_name(&alias) {
        return Err(ProtoError::Message(
            "Versions cannot be aliases. Use alphanumeric words instead.".into(),
        ))?;
    }

    let mut tool = create_tool(&tool_type).await?;

    let manifest = tool.get_manifest_mut()?;
    manifest.aliases.insert(alias.clone(), version.clone());
    manifest.save()?;

    info!(
        "Added alias {} ({}) for {}",
        color::id(alias),
        color::muted_light(version),
        tool.get_name(),
    );

    Ok(())
}
