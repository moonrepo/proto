use crate::helpers::create_theme;
use crate::session::ProtoSession;
use clap::Args;
use dialoguer::Confirm;
use miette::IntoDiagnostic;
use proto_core::{Id, ProtoError, Tool, VersionSpec};
use rustc_hash::FxHashSet;
use starbase::AppResult;
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

    #[arg(long, help = "Avoid and force confirm prompts")]
    pub yes: bool,
}

fn is_older_than_days(now: u128, other: u128, days: u8) -> bool {
    (now - other) > ((days as u128) * 24 * 60 * 60 * 1000)
}

#[tracing::instrument(skip_all)]
pub async fn clean_tool(mut tool: Tool, now: u128, days: u8, yes: bool) -> miette::Result<usize> {
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

    if count == 0 {
        debug!("No versions to remove, continuing to next tool");

        return Ok(0);
    }

    if yes
        || Confirm::with_theme(&create_theme())
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

    for file in fs::read_dir_all(session.env.store.inventory_dir.join("proto"))? {
        let path = file.path();

        if path.is_file() {
            let bytes = fs::remove_file_if_stale(&path, duration, now)?;

            if bytes > 0 {
                debug!(
                    "proto version {} hasn't been used in over {} days, removing",
                    color::path(&path),
                    days
                );

                clean_count += 1;
            }
        }
    }

    Ok(clean_count)
}

#[tracing::instrument(skip(session, yes))]
pub async fn purge_tool(session: &ProtoSession, id: &Id, yes: bool) -> miette::Result<Tool> {
    let tool = session.load_tool(id).await?;
    let inventory_dir = tool.get_inventory_dir();

    if yes
        || Confirm::with_theme(&create_theme())
            .with_prompt(format!(
                "Purge all of {} at {}?",
                tool.get_name(),
                color::path(&inventory_dir)
            ))
            .interact()
            .into_diagnostic()?
    {
        // Delete inventory
        fs::remove_dir_all(inventory_dir)?;

        // Delete binaries
        for bin in tool.get_bin_locations()? {
            session.env.store.unlink_bin(&bin.path)?;
        }

        // Delete shims
        for shim in tool.get_shim_locations()? {
            session.env.store.remove_shim(&shim.path)?;
        }

        println!("Purged {}", tool.get_name());
    }

    Ok(tool)
}

#[tracing::instrument(skip_all)]
pub async fn purge_plugins(session: &ProtoSession, yes: bool) -> AppResult {
    let plugins_dir = &session.env.store.plugins_dir;

    if yes
        || Confirm::with_theme(&create_theme())
            .with_prompt(format!(
                "Purge all plugins in {}?",
                color::path(plugins_dir)
            ))
            .interact()
            .into_diagnostic()?
    {
        fs::remove_dir_all(plugins_dir)?;
        fs::create_dir_all(plugins_dir)?;

        println!("Purged all downloaded plugins");
    }

    Ok(())
}

pub async fn internal_clean(session: &ProtoSession, args: &CleanArgs, yes: bool) -> AppResult {
    let days = args.days.unwrap_or(30);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut clean_count = 0;

    println!("Finding installed tools to clean up...");

    for tool in session.load_tools().await? {
        clean_count += clean_tool(tool, now, days, yes).await?;
    }

    clean_count += clean_proto(session, days as u64).await?;

    if clean_count > 0 {
        println!("Successfully cleaned {} versions", clean_count);
    }

    debug!("Finding installed plugins to clean up...");

    clean_count = clean_plugins(session, days as u64).await?;

    if clean_count > 0 {
        println!("Successfully cleaned up {} plugins", clean_count);
    }

    debug!("Cleaning temporary directory...");

    let results =
        fs::remove_dir_stale_contents(&session.env.store.temp_dir, Duration::from_secs(86400))?;

    if results.files_deleted > 0 {
        println!(
            "Successfully cleaned {} temporary files ({} bytes)",
            results.files_deleted, results.bytes_saved
        );
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn clean(session: ProtoSession, args: CleanArgs) -> AppResult {
    let force_yes = args.yes || !stdout().is_terminal();

    if let Some(id) = &args.purge {
        purge_tool(&session, id, force_yes).await?;
        return Ok(());
    }

    if args.purge_plugins {
        purge_plugins(&session, force_yes).await?;
        return Ok(());
    }

    internal_clean(&session, &args, force_yes).await?;

    Ok(())
}
