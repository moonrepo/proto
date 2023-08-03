use crate::tools::create_tool;
use starbase::SystemResult;
use std::process;
use tracing::debug;

pub async fn list(tool_id: String) -> SystemResult {
    let tool = create_tool(&tool_id).await?;

    debug!(manifest = ?tool.manifest.path, "Using versions from manifest");

    let mut versions = Vec::from_iter(tool.manifest.installed_versions);

    if versions.is_empty() {
        eprintln!("No versions installed");
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
