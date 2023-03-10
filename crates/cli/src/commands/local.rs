use crate::config::Config;
use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use log::{info, trace};
use proto_core::{color, ProtoError};
use std::{env, path::PathBuf};

pub async fn local(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let mut tool = create_tool(&tool_type)?;

    tool.resolve_version(&version).await?;

    let local_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut config = Config::load_from(&local_path)?;

    config
        .tools
        .insert(tool_type, tool.get_resolved_version().to_owned());

    config.save_to(&local_path)?;

    trace!(
        target: "proto:local",
        "Wrote the local version to {}",
        color::path(&local_path),
    );

    info!(
        target: "proto:local",
        "Set the local {} version to {}",
        tool.get_name(),
        tool.get_resolved_version(),
    );

    Ok(())
}
