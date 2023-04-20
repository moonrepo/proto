use crate::tools::{create_tool, ToolType};
use human_sort::compare;
use starbase::SystemResult;
use std::io::{self, Write};
use tracing::debug;

// TODO: only show LTS, dont show pre-releases?
pub async fn list_remote(tool_type: ToolType) -> SystemResult {
    let tool = create_tool(&tool_type).await?;

    debug!("Loading manifest");

    let manifest = tool.load_version_manifest().await?;

    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(stdout);
    let mut releases = manifest.versions.values().collect::<Vec<_>>();

    releases.sort_by(|a, d| compare(&a.version, &d.version));

    for release in releases {
        writeln!(handle, "{}", release.version).unwrap();
    }

    Ok(())
}
