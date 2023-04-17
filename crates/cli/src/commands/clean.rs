use std::time::SystemTime;

use crate::tools::{create_tool, ToolType};
use clap::ValueEnum;
use dialoguer::Confirm;
use proto_core::{color, Manifest};
use rustc_hash::FxHashSet;
use starbase::{diagnostics::IntoDiagnostic, SystemResult};
use starbase_utils::fs;
use tracing::{debug, info};

fn is_older_than_days(now: u128, other: u128, days: u8) -> bool {
    (now - other) > ((days as u128) * 24 * 60 * 60 * 1000)
}

pub async fn clean(days: Option<u8>) -> SystemResult {
    let days = days.unwrap_or(30);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut clean_count = 0;

    info!("Finding tools to clean up...");

    for tool_type in ToolType::value_variants() {
        let mut tool = create_tool(tool_type)?;

        if matches!(tool_type, ToolType::Rust) {
            info!(
                "Skipping {}, use rustup instead",
                color::shell(tool.get_name())
            );

            continue;
        } else {
            info!("Checking {}", color::shell(tool.get_name()));
        }

        if !tool.get_tool_dir().exists() {
            debug!("Not being used, skipping");
            continue;
        }

        let manifest = Manifest::load(tool.get_manifest_path())?;
        let mut versions_to_clean = FxHashSet::default();

        debug!("Scanning file system for stale and untracked versions");

        for dir in fs::read_dir(tool.get_tool_dir())? {
            let dir_path = dir.path();
            let Ok(dir_type) = dir.file_type() else {
                continue;
            };

            if dir_type.is_dir() {
                let version = fs::file_name(&dir_path);

                if version != "globals" && !manifest.versions.contains_key(&version) {
                    debug!("Version {} not found in manifest, removing", version);

                    versions_to_clean.insert(version);
                }
            }
        }

        debug!("Comparing last used timestamps from manifest");

        for (version, metadata) in manifest.versions {
            if versions_to_clean.contains(&version) {
                continue;
            }

            if metadata.no_clean {
                debug!("Version {} is marked as not to clean, skipping", version);
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
                        version, days
                    );

                    versions_to_clean.insert(version);
                }
            }
        }

        let count = versions_to_clean.len();

        if count == 0 {
            debug!("No versions to remove, continuing to next tool");
            continue;
        }

        if Confirm::new()
            .with_prompt(format!(
                "Found {} versions, remove {}?",
                count,
                versions_to_clean
                    .iter()
                    .map(color::id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .interact()
            .into_diagnostic()?
        {
            for version in versions_to_clean {
                tool.set_version(&version);
                tool.teardown().await?;
            }

            clean_count += count;
        } else {
            debug!("Skipping remove, continuing to next tool");
        }
    }

    if clean_count > 0 {
        info!("Successfully cleaned up {} versions", clean_count);
    }

    Ok(())
}
