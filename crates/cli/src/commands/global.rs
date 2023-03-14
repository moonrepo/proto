use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use log::{info, trace};
use proto_core::{color, Manifest, ProtoError};

pub async fn global(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let mut tool = create_tool(&tool_type)?;

    tool.resolve_version(&version).await?;

    let mut manifest = Manifest::load_for_tool(tool.get_bin_name())?;
    manifest.default_version = Some(tool.get_resolved_version().to_owned());
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
        tool.get_resolved_version(),
    );

    Ok(())
}
