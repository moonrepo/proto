use crate::tools::create_tool;
use proto_core::{is_alias_name, ProtoError, VersionType};
use starbase::SystemResult;
use starbase_styles::color;
use tracing::info;

pub async fn alias(tool_id: String, alias: String, version: String) -> SystemResult {
    if alias == version {
        return Err(ProtoError::Message("Cannot map an alias to itself.".into()))?;
    }

    if !is_alias_name(&alias) {
        return Err(ProtoError::Message(
            "Versions cannot be aliases. Use alphanumeric words instead.".into(),
        ))?;
    }

    let mut tool = create_tool(&tool_id).await?;

    tool.manifest
        .aliases
        .insert(alias.clone(), VersionType::parse(&version)?);

    tool.manifest.save()?;

    info!(
        "Added alias {} ({}) for {}",
        color::id(alias),
        color::muted_light(version),
        tool.get_name(),
    );

    Ok(())
}
