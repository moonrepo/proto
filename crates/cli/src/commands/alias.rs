use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use log::info;
use proto_core::{Manifest, ProtoError};

pub async fn alias(tool_type: ToolType, alias: String, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;

    let mut manifest = Manifest::load_for_tool(tool.get_bin_name())?;
    manifest.aliases.insert(alias.clone(), version.clone());
    manifest.save()?;

    info!(
        target: "proto:alias",
        "Added alias {} ({}) for {}",
        alias,
        version,
        tool.get_name(),
    );

    Ok(())
}
