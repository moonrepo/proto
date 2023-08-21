use proto_core::{is_alias_name, load_tool, Id, ProtoError, VersionType};
use starbase::SystemResult;
use starbase_styles::color;
use tracing::info;

pub async fn alias(tool_id: Id, alias: String, version: VersionType) -> SystemResult {
    if let VersionType::Alias(inner_alias) = &version {
        if &alias == inner_alias {
            return Err(ProtoError::Message("Cannot map an alias to itself.".into()))?;
        }
    }

    if !is_alias_name(&alias) {
        return Err(ProtoError::Message(
            "Versions cannot be aliases. Use alphanumeric words instead.".into(),
        ))?;
    }

    let mut tool = load_tool(&tool_id).await?;

    tool.manifest.aliases.insert(alias.clone(), version.clone());
    tool.manifest.save()?;

    info!(
        "Added alias {} ({}) for {}",
        color::id(alias),
        color::muted_light(version.to_string()),
        tool.get_name(),
    );

    Ok(())
}
