use crate::helpers::{enable_logging, get_manifest_path};
use crate::manifest::Manifest;
use crate::tools::{create_tool, ToolType};
use log::{info, trace};
use proto_core::{color, ProtoError};

pub async fn global(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let mut tool = create_tool(&tool_type)?;

    tool.resolve_version(&version).await?;

    let manifest_path = get_manifest_path(&tool)?;

    let mut manifest = Manifest::load(&manifest_path)?;
    manifest.default_version = tool.get_resolved_version().to_owned();
    manifest.save(&manifest_path)?;

    trace!(
        target: "proto:global",
        "Wrote the global version to {}",
        color::path(&manifest_path),
    );

    info!(
        target: "proto:global",
        "Set the global {} version to {}",
        tool.get_name(),
        tool.get_resolved_version(),
    );

    Ok(())
}
