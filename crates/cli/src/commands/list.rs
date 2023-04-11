use crate::tools::{create_tool, ToolType};
use human_sort::compare;
use proto_core::{color, Manifest};
use starbase::SystemResult;
use tracing::debug;

pub async fn list(tool_type: ToolType) -> SystemResult {
    let tool = create_tool(&tool_type)?;
    let manifest = Manifest::load(tool.get_manifest_path())?;

    debug!("Using versions from {}", color::path(&manifest.path));

    let mut versions = Vec::from_iter(manifest.installed_versions);

    if !versions.is_empty() {
        versions.sort_by(|a, d| compare(a, d));

        println!("{}", versions.join("\n"));
    }

    Ok(())
}
