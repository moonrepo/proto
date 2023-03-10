use crate::helpers::enable_logging;
use crate::manifest::Manifest;
use crate::tools::{create_tool, ToolType};
use human_sort::compare;
use log::{debug, info};
use proto_core::{color, ProtoError};

pub async fn list(tool_type: ToolType) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;
    let manifest = Manifest::load_for_tool(&tool)?;

    debug!(target: "proto:list", "Using versions from {}", color::path(&manifest.path));

    info!(target: "proto:list", "Locally installed versions:");

    let mut versions = Vec::from_iter(manifest.installed_versions);

    if versions.is_empty() {
        eprintln!("No versions installed");
    } else {
        versions.sort_by(|a, d| compare(a, d));

        println!("{}", versions.join("\n"));
    }

    Ok(())
}
