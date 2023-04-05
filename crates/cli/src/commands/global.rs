use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use log::{info, trace};
use proto_core::{color, Manifest, ProtoError};

pub async fn global(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;

    let mut manifest = Manifest::load(tool.get_manifest_path())?;
    manifest.default_version = Some(version.clone());
    manifest.save()?;

    trace!(
        target: "proto:global",
        "Wrote the global version to {}",
        color::path(&manifest.path),
    );

    info!(
        target: "proto:global",
        "Set the global {} version to {}",
        tool.get_name(),
        version,
    );

    Ok(())
}
