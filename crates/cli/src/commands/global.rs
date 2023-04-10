use crate::tools::{create_tool, ToolType};
use proto_core::{color, Manifest};
use starbase::SystemResult;
use tracing::{info, trace};

pub async fn global(tool_type: ToolType, version: String) -> SystemResult {
    let tool = create_tool(&tool_type)?;

    let mut manifest = Manifest::load(tool.get_manifest_path())?;
    manifest.default_version = Some(version.clone());
    manifest.save()?;

    trace!(
        "Wrote the global version to {}",
        color::path(&manifest.path),
    );

    info!("Set the global {} version to {}", tool.get_name(), version);

    Ok(())
}
