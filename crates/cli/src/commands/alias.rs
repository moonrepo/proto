use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use log::info;
use proto_core::{color, is_alias_name, Manifest, ProtoError};

pub async fn alias(tool_type: ToolType, alias: String, version: String) -> Result<(), ProtoError> {
    enable_logging();

    if alias == version {
        return Err(ProtoError::Message("Cannot map an alias to itself.".into()));
    }

    if !is_alias_name(&alias) {
        return Err(ProtoError::Message(
            "Versions cannot be aliases. Use alphanumeric words instead.".into(),
        ));
    }

    let tool = create_tool(&tool_type)?;

    let mut manifest = Manifest::load(tool.get_manifest_path())?;
    manifest.aliases.insert(alias.clone(), version.clone());
    manifest.save()?;

    info!(
        target: "proto:alias",
        "Added alias {} ({}) for {}",
        color::id(alias),
        color::muted_light(version),
        tool.get_name(),
    );

    Ok(())
}
