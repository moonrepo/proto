use crate::tools::create_tool;
use proto_core::AliasOrVersion;
use starbase::SystemResult;
use starbase_styles::color;
use tracing::{debug, info};

pub async fn global(tool_id: String, version: String) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;

    tool.manifest.default_version = Some(AliasOrVersion::parse(&version)?);
    tool.manifest.save()?;

    debug!(
        manifest = ?tool.manifest.path,
        "Wrote the global version",
    );

    info!(
        "Set the global {} version to {}",
        tool.get_name(),
        color::hash(version)
    );

    Ok(())
}
