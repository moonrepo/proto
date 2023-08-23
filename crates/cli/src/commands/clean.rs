use dialoguer::Confirm;
use proto_core::{load_tool, AliasOrVersion, Tool, ToolsConfig};
use semver::Version;
use starbase::{diagnostics::IntoDiagnostic, SystemResult};
use starbase_styles::color;
use starbase_utils::fs;
use std::collections::HashSet;
use std::time::SystemTime;
use tracing::{debug, info};

fn is_older_than_days(now: u128, other: u128, days: u8) -> bool {
    (now - other) > ((days as u128) * 24 * 60 * 60 * 1000)
}

pub async fn do_clean(mut tool: Tool, now: u128, days: u8, yes: bool) -> miette::Result<usize> {
    info!("Checking {}", color::shell(tool.get_name()));

    if !tool.get_inventory_dir().exists() {
        debug!("Not being used, skipping");

        return Ok(0);
    }

    let mut versions_to_clean = HashSet::<Version>::new();

    debug!("Scanning file system for stale and untracked versions");

    for dir in fs::read_dir(tool.get_inventory_dir())? {
        let dir_path = dir.path();

        let Ok(dir_type) = dir.file_type() else {
            continue;
        };

        if dir_type.is_dir() {
            let dir_name = fs::file_name(&dir_path);

            if dir_name == "globals" {
                continue;
            }

            let version = Version::parse(&dir_name).into_diagnostic()?;

            if !tool.manifest.versions.contains_key(&version) {
                debug!(
                    "Version {} not found in manifest, removing",
                    color::hash(version.to_string())
                );

                versions_to_clean.insert(version);
            }
        }
    }

    debug!("Comparing last used timestamps from manifest");

    for (version, metadata) in &tool.manifest.versions {
        if versions_to_clean.contains(version) {
            continue;
        }

        if metadata.no_clean {
            debug!(
                "Version {} is marked as not to clean, skipping",
                color::hash(version.to_string())
            );

            continue;
        }

        // None may mean a few things:
        // - It was recently installed but not used yet
        // - It was installed before we started tracking last used timestamps
        // - The tools run via external commands (e.g. moon)
        if let Some(last_used) = metadata.last_used_at {
            if is_older_than_days(now, last_used, days) {
                debug!(
                    "Version {} hasn't been used in over {} days, removing",
                    color::hash(version.to_string()),
                    days
                );

                versions_to_clean.insert(version.to_owned());
            }
        }
    }

    let count = versions_to_clean.len();
    let mut clean_count = 0;

    if count == 0 {
        debug!("No versions to remove, continuing to next tool");

        return Ok(0);
    }

    if yes
        || Confirm::new()
            .with_prompt(format!(
                "Found {} versions, remove {}?",
                count,
                versions_to_clean
                    .iter()
                    .map(|v| color::hash(v.to_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .interact()
            .into_diagnostic()?
    {
        for version in versions_to_clean {
            tool.set_version(AliasOrVersion::Version(version));
            tool.teardown().await?;
        }

        clean_count += count;
    } else {
        debug!("Skipping remove, continuing to next tool");
    }

    Ok(clean_count)
}

pub async fn clean(days: Option<u8>, yes: bool) -> SystemResult {
    let days = days.unwrap_or(30);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut clean_count = 0;

    info!("Finding tools to clean up...");

    let mut tools_config = ToolsConfig::load_upwards()?;
    tools_config.inherit_builtin_plugins();

    if !tools_config.plugins.is_empty() {
        for id in tools_config.plugins.keys() {
            clean_count += do_clean(load_tool(id).await?, now, days, yes).await?;
        }
    }

    if clean_count > 0 {
        info!("Successfully cleaned up {} versions", clean_count);
    }

    Ok(())
}
