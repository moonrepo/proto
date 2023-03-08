use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use human_sort::compare;
use log::{debug, info};
use proto_core::{color, ProtoError};
use std::fs;

pub async fn list(tool_type: ToolType) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;
    let install_dir = tool.get_install_dir()?;
    let tool_dir = install_dir.parent().unwrap(); // Without version/latest

    debug!(target: "proto:list", "Finding versions in {}", color::path(tool_dir));

    let handle_error = |e: std::io::Error| ProtoError::Fs(tool_dir.to_path_buf(), e.to_string());
    let mut versions = vec![];

    if tool_dir.exists() {
        for entry in fs::read_dir(tool_dir).map_err(handle_error)? {
            let entry = entry.map_err(handle_error)?;

            if entry.file_type().map_err(handle_error)?.is_dir() {
                versions.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }

    info!(target: "proto:list", "Locally installed versions:");

    if versions.is_empty() {
        eprintln!("No versions installed");
    } else {
        versions.sort_by(|a, d| compare(a, d));

        println!("{}", versions.join("\n"));
    }

    Ok(())
}
