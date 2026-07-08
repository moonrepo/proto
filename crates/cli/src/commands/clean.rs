use crate::helpers::join_list;
use crate::session::{ProtoSession, SessionResult};
use clap::{Args, ValueEnum};
use iocraft::prelude::element;
use proto_core::ToolSpec;
use proto_core::flow::manage::Manager;
use proto_core::reporter::NoticeOutput;
use proto_core::{PROTO_PLUGIN_KEY, Tool, VersionSpec, flow::resolve::ProtoResolveError};
use proto_shim::get_exe_file_name;
use rustc_hash::FxHashSet;
use serde::Serialize;
use starbase_console::ui::*;
use starbase_console::utils::formats::format_bytes_binary;
use starbase_styles::color;
use starbase_utils::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, instrument};

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum CleanTarget {
    #[default]
    All,
    Cache,
    Plugins,
    Temp,
    Tools,
}

#[derive(Args, Clone, Debug, Default)]
pub struct CleanArgs {
    #[arg(value_enum, default_value_t, help = "Specific target to clean")]
    pub target: CleanTarget,

    #[arg(
        long,
        default_value_t = 30,
        help = "Clean tools and plugins older than the specified number of days"
    )]
    pub days: u8,
}

#[derive(Default, Serialize)]
pub struct CleanResult {
    cache: Vec<StaleFile>,
    plugins: Vec<StaleFile>,
    temp: Vec<StaleFile>,
    tools: Vec<StaleTool>,
}

#[derive(Serialize)]
pub struct StaleTool {
    dir: PathBuf,
    id: String,
    version: VersionSpec,
}

#[derive(Serialize)]
pub struct StaleFile {
    file: PathBuf,
    size: u64,
}

fn is_older_than_days(now: u128, other: u128, days: u64) -> bool {
    (now - other) > ((days as u128) * 24 * 60 * 60 * 1000)
}

#[instrument(skip(session))]
pub async fn clean_tool(
    session: &ProtoSession,
    mut tool: Tool,
    now: SystemTime,
    days: u64,
) -> miette::Result<Vec<StaleTool>> {
    let mut cleaned = vec![];

    debug!("Checking {}", tool.get_name());

    if tool.metadata.inventory_options.override_dir.is_some() {
        debug!("Using an external inventory, skipping");

        return Ok(cleaned);
    }

    let inventory_dir = tool.get_inventory_dir().to_path_buf();

    if !inventory_dir.exists() {
        debug!("Not being used, skipping");

        return Ok(cleaned);
    }

    let mut versions_to_clean = FxHashSet::<VersionSpec>::default();
    let now_millis = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

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

            let version = VersionSpec::parse(&dir_name).map_err(|error| {
                ProtoResolveError::InvalidVersionSpec {
                    version: dir_name,
                    error: Box::new(error),
                }
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
        if let Ok(Some(last_used)) = tool.inventory.create_product(version).load_used_at()
            && is_older_than_days(now_millis, last_used, days)
        {
            debug!(
                "Version {} hasn't been used in over {} days, removing",
                color::hash(version.to_string()),
                days
            );

            versions_to_clean.insert(version.to_owned());
        }
    }

    if versions_to_clean.is_empty() {
        debug!("No versions to remove, continuing to next tool");

        return Ok(cleaned);
    }

    let skip_prompts = session.should_skip_prompts();
    let mut confirmed = false;

    if !skip_prompts {
        session
            .console
            .render_interactive(element! {
                Confirm(
                    label: format!(
                        "Found {} stale {} versions, remove {}?",
                        versions_to_clean.len(),
                        tool.get_name(),
                        join_list(
                            versions_to_clean
                                .iter()
                                .map(|v| format!("<version>{v}</version>"))
                                .collect::<Vec<_>>()
                        )
                    ),
                    on_confirm: &mut confirmed,
                )
            })
            .await?;
    }

    if skip_prompts || confirmed {
        let tool_id = tool.get_id().to_string();
        let mut manager = Manager::new(&mut tool);

        for version in versions_to_clean {
            cleaned.push(StaleTool {
                dir: inventory_dir.join(version.to_string()),
                id: tool_id.clone(),
                version: version.clone(),
            });

            manager
                .uninstall(&mut ToolSpec::new_resolved(version))
                .await?;
        }

        manager.sync_manifest().await?;
    } else {
        debug!("Skipping remove, continuing to next tool");
    }

    Ok(cleaned)
}

#[instrument(skip(session))]
pub async fn clean_proto_tool(session: &ProtoSession, days: u64) -> miette::Result<Vec<StaleTool>> {
    let duration = Duration::from_secs(86400 * days);
    let mut cleaned = vec![];

    for dir in fs::read_dir(session.env.store.inventory_dir.join(PROTO_PLUGIN_KEY))? {
        let tool_dir = dir.path();

        // Ignore hidden files
        if !tool_dir.is_dir() {
            continue;
        }

        let proto_file = tool_dir.join(get_exe_file_name("proto"));
        let dir_name = fs::file_name(&tool_dir);

        let version = VersionSpec::parse(&dir_name).map_err(|error| {
            ProtoResolveError::InvalidVersionSpec {
                version: dir_name,
                error: Box::new(error),
            }
        })?;

        let is_stale = if proto_file.exists() {
            fs::is_stale(proto_file, true, duration)?
        } else {
            true
        };

        if is_stale {
            debug!(
                "proto version {} hasn't been used in over {} days, removing",
                color::path(&tool_dir),
                days
            );

            fs::remove_dir_all(&tool_dir)?;

            cleaned.push(StaleTool {
                dir: tool_dir,
                id: PROTO_PLUGIN_KEY.to_owned(),
                version,
            });
        }
    }

    Ok(cleaned)
}

#[instrument]
pub fn clean_dir(dir: &Path, days: u64, recurse: bool) -> miette::Result<Vec<StaleFile>> {
    let duration = Duration::from_secs(86400 * days);
    let mut cleaned = vec![];

    for entry in fs::read_dir(dir)? {
        let path = entry.path();

        // Don't delete dot files/folders
        if path
            .file_name()
            .is_some_and(|name| name.as_encoded_bytes().starts_with(b"."))
        {
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_file() {
            if let Some(bytes) = fs::remove_file_if_stale(&path, duration)?
                && bytes > 0
            {
                debug!(
                    "File {} hasn't been used in over {} days, removing",
                    color::path(&path),
                    days
                );

                cleaned.push(StaleFile {
                    file: path,
                    size: bytes,
                })
            }
        } else if recurse && file_type.is_dir() {
            cleaned.extend(clean_dir(&path, days, recurse)?);

            // Prune the subdirectory if empty; a concurrent clean or write is not an error
            match std::fs::remove_dir(&path) {
                Ok(_) => {
                    debug!("Directory {} is now empty, removing", color::path(&path));
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::DirectoryNotEmpty
                    ) => {}
                Err(error) => {
                    return Err(fs::FsError::Remove {
                        path,
                        error: Box::new(error),
                    }
                    .into());
                }
            }
        }
    }

    Ok(cleaned)
}

#[instrument(skip(session))]
pub async fn internal_clean(
    session: &ProtoSession,
    args: &CleanArgs,
) -> miette::Result<CleanResult> {
    let days = args.days as u64;
    let now = SystemTime::now();
    let mut result = CleanResult::default();

    if matches!(args.target, CleanTarget::All | CleanTarget::Tools) {
        debug!("Cleaning installed tools...");

        for tool in session.load_all_tools().await? {
            if tool.get_id() == PROTO_PLUGIN_KEY {
                continue;
            }

            result
                .tools
                .extend(clean_tool(session, tool.tool, now, days).await?);
        }

        // proto has special handling
        result.tools.extend(clean_proto_tool(session, days).await?);
    }

    if matches!(args.target, CleanTarget::All | CleanTarget::Plugins) {
        debug!("Cleaning downloaded plugins...");

        // Plugins are stored as flat files, so no recursion is needed
        result.plugins = clean_dir(&session.env.store.plugins_dir, days, false)?;
    }

    if matches!(args.target, CleanTarget::All | CleanTarget::Temp) {
        debug!("Cleaning temporary directory...");

        // Temp contents are coordinated through the install and plugin download
        // locks, and both flows remove their own directories when they finish;
        // recursive cleanup would have to participate in those lock protocols,
        // so keep the previous top-level behavior here
        result.temp = clean_dir(&session.env.store.temp_dir, days, false)?;
    }

    if matches!(args.target, CleanTarget::All | CleanTarget::Cache) {
        debug!("Cleaning cache directory...");

        result.cache = clean_dir(&session.env.store.cache_dir, days, true)?;
    }

    Ok(result)
}

#[instrument(skip(session))]
pub async fn clean(session: ProtoSession, args: CleanArgs) -> SessionResult {
    let result = internal_clean(&session, &args).await?;

    if session.is_json_format() {
        session.console.write_json_for_format(result)?;

        return Ok(None);
    }

    let remove_count =
        result.cache.len() + result.plugins.len() + result.temp.len() + result.tools.len();

    if remove_count == 0 {
        session.console.notice(
            Variant::Info,
            format!(
                "Clean complete but nothing was removed.\nNo artifacts were found older than {} days.",
                args.days
            ),
        )?;
    } else {
        let mut items = vec![];

        if !result.cache.is_empty() {
            items.push(format!(
                "{} cached items ({})",
                result.cache.len(),
                format_bytes_binary(result.cache.iter().fold(0, |acc, x| acc + x.size))
            ));
        }

        if !result.plugins.is_empty() {
            items.push(format!(
                "{} downloaded plugins ({})",
                result.plugins.len(),
                format_bytes_binary(result.plugins.iter().fold(0, |acc, x| acc + x.size))
            ));
        }

        if !result.temp.is_empty() {
            items.push(format!(
                "{} temporary files ({})",
                result.temp.len(),
                format_bytes_binary(result.temp.iter().fold(0, |acc, x| acc + x.size))
            ));
        }

        if !result.tools.is_empty() {
            items.push(format!("{} installed tool versions", result.tools.len()));
        }

        session.console.notice_with(NoticeOutput {
            variant: Variant::Success,
            messages: vec![format!(
                "Clean complete, {} artifacts removed:",
                remove_count
            )],
            items,
            ..Default::default()
        })?;
    }

    Ok(None)
}
