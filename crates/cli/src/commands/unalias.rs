use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use log::info;
use proto_core::{color, Manifest, ProtoError};

pub async fn unalias(tool_type: ToolType, alias: String) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;

    let mut manifest = Manifest::load_for_tool(tool.get_bin_name())?;
    let value = manifest.aliases.remove(&alias);
    manifest.save()?;

    if let Some(version) = value {
        info!(
            target: "proto:unalias",
            "Removed alias {} ({}) from {}",
            color::id(alias),
            color::muted_light(version),
            tool.get_name(),
        );
    } else {
        info!(
            target: "proto:unalias",
            "Alias {} not found for {}",
            color::id(alias),
            tool.get_name(),
        );
    }

    Ok(())
}
