use crate::tools::create_tool;
use proto_core::{AliasOrVersion, Id};
use starbase::SystemResult;
use starbase_styles::color;
use tracing::{debug, info};

pub async fn global(tool_id: Id, version: AliasOrVersion) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;

    tool.manifest.default_version = Some(version.clone());
    tool.manifest.save()?;

    debug!(
        version = version.to_string(),
        manifest = ?tool.manifest.path,
        "Wrote the global version",
    );

    info!(
        "Set the global {} version to {}",
        tool.get_name(),
        color::hash(version.to_string())
    );

    Ok(())
}
