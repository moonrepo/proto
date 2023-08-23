use proto_core::{load_tool, Id, VersionType};
use starbase::SystemResult;
use std::process;
use tracing::debug;

// TODO: only show LTS, dont show pre-releases?
pub async fn list_remote(tool_id: Id) -> SystemResult {
    let tool = load_tool(&tool_id).await?;

    debug!("Loading versions");

    let resolver = tool.load_version_resolver(&VersionType::default()).await?;
    let mut versions = resolver.versions;

    if versions.is_empty() {
        eprintln!("No versions available");
        process::exit(1);
    }

    versions.sort();

    println!(
        "{}",
        versions
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );

    Ok(())
}
