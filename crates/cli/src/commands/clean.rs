use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, ProtoError, Tool, VersionSpec, PROTO_PLUGIN_KEY};
use proto_shim::get_exe_file_name;
use rustc_hash::FxHashSet;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_styles::color;
use starbase_utils::fs;
use std::io::stdout;
use std::io::IsTerminal;
use std::time::{Duration, SystemTime};
use tracing::debug;

#[derive(Args, Clone, Debug, Default)]
pub struct CleanArgs {
    #[arg(
        long,
        help = "Clean tools and plugins older than the specified number of days"
    )]
    pub days: Option<u8>,

    #[arg(
        long,
        help = "Purge and delete the installed tool by ID",
        group = "purge-type",
        value_name = "TOOL"
    )]
    pub purge: Option<Id>,

    #[arg(
        long,
        help = "Purge and delete all installed plugins",
        group = "purge-type"
    )]
    pub purge_plugins: bool,

    #[arg(long, help = "Avoid and force confirm prompts", env = "PROTO_YES")]
    pub yes: bool,
}

fn is_older_than_days(now: u128, other: u128, days: u8) -> bool {
    (now - other) > ((days as u128) * 24 * 60 * 60 * 1000)
}

#[tracing::instrument(skip_all)]
pub async fn clean_tool(
    session: &ProtoSession,
    mut tool: Tool,
    now: u128,
    days: u8,
    yes: bool,
) -> miette::Result<usize> {
    println!("Checking {}", color::shell(tool.get_name()));

    if tool.metadata.inventory.override_dir.is_some() {
        debug!("Using an external inventory, skipping");

        return Ok(0);
    }

    let inventory_dir = tool.get_inventory_dir();

    if !inventory_dir.exists() {
        debug!("Not being used, skipping");

        return Ok(0);
    }

    let mut versions_to_clean = FxHashSet::<VersionSpec>::default();

    debug!("Scanning file system for stale and untracked versions");

    for dir in fs::read_dir(&inventory_dir)? {
        let dir_path = dir.path();

        let Ok(dir_type) = dir.file_type() else {
            continue;
        };

        if dir_type.is_dir() {
            let dir_name = fs::file_name(&dir_path);

            // Node.js compat
            if dir_name == "globals" {
                continue;
            }

            let version =
                VersionSpec::parse(&dir_name).map_err(|error| ProtoError::VersionSpec {
                    version: dir_name,
                    error: Box::new(error),
                })?;

            if !tool.inventory.manifest.versions.contains_key(&version) {
                debug!(
                    "Version {} not found in manifest, removing",
                    color::hash(version.to_string())
                );

                versions_to_clean.insert(version);
            }
        }
    }

    debug!("Comparing last used timestamps from manifest");

    for (version, metadata) in &tool.inventory.manifest.versions {
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
        if let Ok(Some(last_used)) = tool.inventory.create_product(version).load_used_at() {
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
    let mut confirmed = false;

    if count == 0 {
        debug!("No versions to remove, continuing to next tool");

        return Ok(0);
    }

    session
        .console
        .render_interactive(element! {
            Confirm(
                label: format!(
                    "Found {} versions, remove {}?",
                    count,
                    versions_to_clean
                        .iter()
                        .map(|v| format!("<hash>{v}</hash>"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                value: &mut confirmed,
            )
        })
        .await?;

    if yes || confirmed {
        for version in versions_to_clean {
            tool.set_version(version);
            tool.teardown().await?;
        }

        clean_count += count;
    } else {
        debug!("Skipping remove, continuing to next tool");
    }

    Ok(clean_count)
}

#[tracing::instrument(skip_all)]
pub async fn clean_plugins(session: &ProtoSession, days: u64) -> miette::Result<usize> {
    let now = SystemTime::now();
    let duration = Duration::from_secs(86400 * days);
    let mut clean_count = 0;

    for file in fs::read_dir(&session.env.store.plugins_dir)? {
        let path = file.path();

        if path.is_file() {
            let bytes = fs::remove_file_if_stale(&path, duration, now)?;

            if bytes > 0 {
                debug!(
                    "Plugin {} hasn't been used in over {} days, removing",
                    color::path(&path),
                    days
                );

                clean_count += 1;
            }
        }
    }

    Ok(clean_count)
}

#[tracing::instrument(skip_all)]
pub async fn clean_proto(session: &ProtoSession, days: u64) -> miette::Result<usize> {
    let now = SystemTime::now();
    let duration = Duration::from_secs(86400 * days);
    let mut clean_count = 0;

    for dir in fs::read_dir(session.env.store.inventory_dir.join(PROTO_PLUGIN_KEY))? {
        let tool_dir = dir.path();

        // Ignore hidden files
        if !tool_dir.is_dir() {
            continue;
        }

        let proto_file = tool_dir.join(get_exe_file_name("proto"));

        let is_stale = if proto_file.exists() {
            fs::is_stale(proto_file, false, duration, now)?.is_some()
        } else {
            true
        };

        if is_stale {
            debug!(
                "proto version {} hasn't been used in over {} days, removing",
                color::path(&tool_dir),
                days
            );

            fs::remove_dir_all(tool_dir)?;
            clean_count += 1;
        }
    }

    Ok(clean_count)
}

#[tracing::instrument(skip(session, yes))]
pub async fn purge_tool(session: &ProtoSession, id: &Id, yes: bool) -> miette::Result<Tool> {
    let tool = session.load_tool(id).await?;
    let inventory_dir = tool.get_inventory_dir();
    let mut confirmed = false;

    session
        .console
        .render_interactive(element! {
            Confirm(
                label: format!(
                    "Purge all of {} at <path>{}</path>?",
                    tool.get_name(),
                    inventory_dir.display()
                ),
                value: &mut confirmed,
            )
        })
        .await?;

    if yes || confirmed {
        // Delete inventory
        fs::remove_dir_all(inventory_dir)?;

        // Delete binaries
        for bin in tool.resolve_bin_locations(true).await? {
            session.env.store.unlink_bin(&bin.path)?;
        }

        // Delete shims
        for shim in tool.resolve_shim_locations().await? {
            session.env.store.remove_shim(&shim.path)?;
        }

        println!("Purged {}", tool.get_name());
    }

    Ok(tool.tool)
}

#[tracing::instrument(skip_all)]
pub async fn purge_plugins(session: &ProtoSession, yes: bool) -> AppResult {
    let plugins_dir = &session.env.store.plugins_dir;
    let mut confirmed = false;

    session
        .console
        .render_interactive(element! {
            Confirm(
                label: format!(
                    "Purge all plugins in <path>{}</path>?",
                    plugins_dir.display()
                ),
                value: &mut confirmed,
            )
        })
        .await?;

    if yes || confirmed {
        fs::remove_dir_all(plugins_dir)?;
        fs::create_dir_all(plugins_dir)?;

        println!("Purged all downloaded plugins");
    }

    Ok(None)
}

pub async fn internal_clean(
    session: &ProtoSession,
    args: CleanArgs,
    yes: bool,
    log: bool,
) -> AppResult {
    let days = args.days.unwrap_or(30);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut clean_count = 0;

    debug!("Finding installed tools to clean up...");

    for tool in session.load_tools().await? {
        clean_count += clean_tool(session, tool.tool, now, days, yes).await?;
    }

    clean_count += clean_proto(session, days as u64).await?;

    if log && clean_count > 0 {
        println!("Successfully cleaned {} versions", clean_count);
    }

    debug!("Finding installed plugins to clean up...");

    clean_count = clean_plugins(session, days as u64).await?;

    if log && clean_count > 0 {
        println!("Successfully cleaned up {} plugins", clean_count);
    }

    debug!("Cleaning temporary directory...");

    let results =
        fs::remove_dir_stale_contents(&session.env.store.temp_dir, Duration::from_secs(86400))?;

    if log && results.files_deleted > 0 {
        println!(
            "Successfully cleaned {} temporary files ({} bytes)",
            results.files_deleted, results.bytes_saved
        );
    }

    debug!("Cleaning cache directory...");

    let results =
        fs::remove_dir_stale_contents(&session.env.store.cache_dir, Duration::from_secs(86400))?;

    if log && results.files_deleted > 0 {
        println!(
            "Successfully cleaned {} cache files ({} bytes)",
            results.files_deleted, results.bytes_saved
        );
    }

    Ok(None)
}

#[tracing::instrument(skip_all)]
pub async fn clean(session: ProtoSession, args: CleanArgs) -> AppResult {
    let force_yes = args.yes || !stdout().is_terminal();

    if let Some(id) = &args.purge {
        purge_tool(&session, id, force_yes).await?;
        return Ok(None);
    }

    if args.purge_plugins {
        purge_plugins(&session, force_yes).await?;
        return Ok(None);
    }

    internal_clean(&session, args, force_yes, true).await?;

    Ok(None)
}
