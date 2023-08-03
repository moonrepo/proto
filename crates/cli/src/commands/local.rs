use crate::tools::create_tool;
use proto_core::{AliasOrVersion, ToolsConfig};
use starbase::SystemResult;
use starbase_styles::color;
use std::{env, path::PathBuf};
use tracing::{debug, info};

pub async fn local(tool_id: String, version: String) -> SystemResult {
    let tool = create_tool(&tool_id).await?;
    let local_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut config = ToolsConfig::load_from(&local_path)?;

    config
        .tools
        .insert(tool.id.clone(), AliasOrVersion::parse(&version)?);

    config.save()?;

    debug!(config = ?local_path, "Wrote the local version");

    info!(
        "Set the local {} version to {}",
        tool.get_name(),
        color::hash(version)
    );

    Ok(())
}
