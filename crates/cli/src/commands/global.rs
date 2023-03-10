use crate::helpers::enable_logging;
use crate::manifest::{Manifest, MANIFEST_NAME};
use crate::tools::{create_tool, ToolType};
use log::{info, trace};
use proto_core::{color, get_tools_dir, ProtoError};

pub async fn global(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let mut tool = create_tool(&tool_type)?;

    tool.resolve_version(&version).await?;

    let manifest_path = get_tools_dir()?
        .join(tool.get_bin_name())
        .join(MANIFEST_NAME);

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
