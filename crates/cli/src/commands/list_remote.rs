use crate::tools::create_tool;
use proto_core::AliasOrVersion;
use starbase::SystemResult;
use std::process;
use tracing::debug;

// TODO: only show LTS, dont show pre-releases?
pub async fn list_remote(tool_id: String) -> SystemResult {
    let tool = create_tool(&tool_id).await?;

    debug!("Loading versions");

    let resolver = tool
        .load_version_resolver(&AliasOrVersion::default())
        .await?;
    let mut versions = resolver.versions;

    if versions.is_empty() {
        eprintln!("No versions available");
        process::exit(1);
    }

    versions.sort_by(|a, d| a.cmp(d));

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
