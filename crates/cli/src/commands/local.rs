use proto_core::{load_tool, AliasOrVersion, Id, ToolsConfig};
use starbase::SystemResult;
use starbase_styles::color;
use std::{env, path::PathBuf};
use tracing::{debug, info};

pub async fn local(tool_id: Id, version: AliasOrVersion) -> SystemResult {
    let tool = load_tool(&tool_id).await?;
    let local_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut config = ToolsConfig::load_from(local_path)?;
    config.tools.insert(tool_id, version.clone());
    config.save()?;

    debug!(
        version = version.to_string(),
        config = ?config.path,
        "Wrote the local version",
    );

    info!(
        "Set the local {} version to {}",
        tool.get_name(),
        color::hash(version.to_string())
    );

    Ok(())
}
