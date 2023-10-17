use crate::helpers::load_configured_tools;
use proto_core::get_bin_dir;
use starbase::SystemResult;
use starbase_utils::fs;
use std::collections::HashSet;
use tracing::{debug, info};

pub async fn migrate() -> SystemResult {
    info!("Loading tools...");

    let mut tools = vec![];

    load_configured_tools(HashSet::new(), |tool, _| {
        // Skips tools/plugins that are not in use
        if !tool.manifest.installed_versions.is_empty() {
            tools.push(tool);
        }
    })
    .await?;

    for tool in &mut tools {
        // Resolve the global version for use in shims and bins
        if let Some(spec) = tool.manifest.default_version.clone() {
            tool.resolve_version(&spec).await?;
        }
    }

    info!("Deleting old shims...");

    for file in fs::read_dir(get_bin_dir()?)? {
        let path = file.path();
        let name = fs::file_name(&path);

        if name == "proto" || name == "proto.exe" || name == "moon" || name == "moon.exe" {
            continue;
        }

        debug!(shim = ?path, "Deleting shim");

        fs::remove_file(path)?;
    }

    info!("Generating new shims...");

    for tool in &mut tools {
        // Always create shims for all active tools
        tool.setup_shims(true).await?;
    }

    info!("Linking new binaries...");

    for tool in &mut tools {
        // Only the global version is linked, so only create if set
        if tool.manifest.default_version.is_some() {
            tool.setup_bin_link(true)?;
        }
    }

    info!("Updating shell profile...");

    info!("Migration complete!");

    Ok(())
}
