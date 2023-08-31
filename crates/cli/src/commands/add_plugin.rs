use proto_core::{load_tool, Id};
use starbase::SystemResult;
use std::process;
use tracing::debug;

pub async fn add_plugin(tool_id: Id) -> SystemResult {
    let tool = load_tool(&tool_id).await?;

    debug!(manifest = ?tool.manifest.path, "Using versions from manifest");

    let mut versions = Vec::from_iter(tool.manifest.installed_versions);

    if versions.is_empty() {
        eprintln!("No versions installed");
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
