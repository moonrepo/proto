use clap::Args;
use dialoguer::Confirm;
use proto_core::{get_plugins_dir, load_tool, AliasOrVersion, Id, Tool, ToolsConfig};
use semver::Version;
use starbase::diagnostics::IntoDiagnostic;
use starbase::{system, SystemResult};
use starbase_styles::color;
use starbase_utils::fs;
use std::collections::HashSet;
use std::time::SystemTime;
use tracing::{debug, info};

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

pub async fn clean_tool(mut tool: Tool, now: u128, days: u8, yes: bool) -> miette::Result<usize> {
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

pub async fn clean_plugins(now: u128, days: u8) -> miette::Result<usize> {
    let mut clean_count = 0;

    for file in fs::read_dir(get_plugins_dir()?)? {
        let path = file.path();

        if path.is_file() {
            let metadata = fs::metadata(&path)?;
            let last_used = metadata
                .accessed()
                .or_else(|_| metadata.modified())
                .or_else(|_| metadata.created())
                .unwrap_or_else(|_| SystemTime::now())
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis();

            if is_older_than_days(now, last_used, days) {
                debug!(
                    "Plugin {} hasn't been used in over {} days, removing",
                    color::path(&path),
                    days
                );

                fs::remove_file(path)?;
                clean_count += 1;
            }
        }
    }

    Ok(clean_count)
}

pub async fn internal_clean(args: &CleanArgs) -> SystemResult {
    let days = args.days.unwrap_or(30);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut clean_count = 0;

    info!("Finding installed tools to clean up...");

    let mut tools_config = ToolsConfig::load_upwards()?;
    tools_config.inherit_builtin_plugins();

    if !tools_config.plugins.is_empty() {
        for id in tools_config.plugins.keys() {
            clean_count += clean_tool(load_tool(id).await?, now, days, args.yes).await?;
        }
    }

    if clean_count > 0 {
        info!("Successfully cleaned up {} versions", clean_count);
    }

    info!("Finding installed plugins to clean up...");

    clean_count = clean_plugins(now, days).await?;

    if clean_count > 0 {
        info!("Successfully cleaned up {} plugins", clean_count);
    }

    Ok(())
}

#[system]
pub async fn clean(args: ArgsRef<CleanArgs>) {
    if let Some(id) = &args.purge {
        let tool = load_tool(id).await?;
        let inventory_dir = tool.get_inventory_dir();

        if args.yes
            || Confirm::new()
                .with_prompt(format!(
                    "Purge all of {} at {}?",
                    tool.get_name(),
                    color::path(&inventory_dir)
                ))
                .interact()
                .into_diagnostic()?
        {
            fs::remove_dir_all(inventory_dir)?;

            info!("Removed {}", tool.get_name());
        }

        return Ok(());
    }

    if args.purge_plugins {
        let plugins_dir = get_plugins_dir()?;

        if args.yes
            || Confirm::new()
                .with_prompt(format!(
                    "Purge all plugins in {}?",
                    color::path(&plugins_dir)
                ))
                .interact()
                .into_diagnostic()?
        {
            fs::remove_dir_all(&plugins_dir)?;
            fs::create_dir_all(plugins_dir)?;

            info!("Removed all installed plugins");
        }

        return Ok(());
    }

    internal_clean(args).await?;
}
