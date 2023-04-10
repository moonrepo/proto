use crate::tools::{create_tool, ToolType};
use proto_core::{color, ToolsConfig};
use starbase::SystemResult;
use std::{env, path::PathBuf};
use tracing::{info, trace};

pub async fn local(tool_type: ToolType, version: String) -> SystemResult {
    let tool = create_tool(&tool_type)?;

    let local_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut config = ToolsConfig::load_from(&local_path)?;

    config
        .tools
        .insert(tool.get_bin_name().to_owned(), version.clone());

    config.save()?;

    trace!("Wrote the local version to {}", color::path(&local_path));

    info!("Set the local {} version to {}", tool.get_name(), version);

    Ok(())
}
