use crate::helpers::enable_logging;
use crate::tools::{create_tool, ToolType};
use proto_core::{color, ProtoError, ToolsConfig};
use std::{env, path::PathBuf};
use tracing::{info, trace};

pub async fn local(tool_type: ToolType, version: String) -> Result<(), ProtoError> {
    enable_logging();

    let tool = create_tool(&tool_type)?;

    let local_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut config = ToolsConfig::load_from(&local_path)?;

    config
        .tools
        .insert(tool.get_bin_name().to_owned(), version.clone());

    config.save()?;

    trace!("Wrote the local version to {}", color::path(&local_path),);

    info!("Set the local {} version to {}", tool.get_name(), version,);

    Ok(())
}
