use crate::helpers::join_list;
use crate::session::ProtoSession;
use clap::{Args, ValueEnum};
use iocraft::prelude::element;
use proto_core::ToolSpec;
use proto_core::{PROTO_PLUGIN_KEY, Tool, VersionSpec, flow::resolve::ProtoResolveError};
use proto_shim::get_exe_file_name;
use rustc_hash::FxHashSet;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_styles::color;
use starbase_utils::{fs, json};
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

#[instrument(skip_all)]
pub async fn clean_tool(
    session: &ProtoSession,
    mut tool: Tool,
    now: SystemTime,
    days: u64,
) -> miette::Result<Vec<StaleTool>> {
    let mut cleaned = vec![];

    debug!("Checking {}", tool.get_name());

    if tool.metadata.inventory.override_dir.is_some() {
        debug!("Using an external inventory, skipping");

        return Ok(cleaned);
    }

    let inventory_dir = tool.get_inventory_dir();

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
        for version in versions_to_clean {
            cleaned.push(StaleTool {
                dir: inventory_dir.join(version.to_string()),
                id: tool.get_id().to_string(),
                version: version.clone(),
            });

            tool.teardown(&ToolSpec::new(version.to_unresolved_spec()))
                .await?;
        }
    } else {
        debug!("Skipping remove, continuing to next tool");
    }

    Ok(cleaned)
}

#[instrument(skip_all)]
pub async fn clean_proto_tool(
    session: &ProtoSession,
    now: SystemTime,
    days: u64,
) -> miette::Result<Vec<StaleTool>> {
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
            fs::is_stale(proto_file, true, duration, now)?.is_some()
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

#[instrument(skip_all)]
pub async fn clean_dir(dir: &Path, now: SystemTime, days: u64) -> miette::Result<Vec<StaleFile>> {
    let duration = Duration::from_secs(86400 * days);
    let mut cleaned = vec![];

    for file in fs::read_dir(dir)? {
        let path = file.path();

        if path.is_file() {
            let bytes = fs::remove_file_if_stale(&path, duration, now)?;

            if bytes > 0 {
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
        }
    }

    Ok(cleaned)
}

#[instrument(skip_all)]
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
        result
            .tools
            .extend(clean_proto_tool(session, now, days).await?);
    }

    if matches!(args.target, CleanTarget::All | CleanTarget::Plugins) {
        debug!("Cleaning downloaded plugins...");

        result.plugins = clean_dir(&session.env.store.plugins_dir, now, days).await?;
    }

    if matches!(args.target, CleanTarget::All | CleanTarget::Temp) {
        debug!("Cleaning temporary directory...");

        result.temp = clean_dir(&session.env.store.temp_dir, now, days).await?;
    }

    if matches!(args.target, CleanTarget::All | CleanTarget::Cache) {
        debug!("Cleaning cache directory...");

        result.cache = clean_dir(&session.env.store.cache_dir, now, days).await?;
    }

    Ok(result)
}

#[instrument(skip_all)]
pub async fn clean(session: ProtoSession, args: CleanArgs) -> AppResult {
    let result = internal_clean(&session, &args).await?;

    if session.should_print_json() {
        session
            .console
            .out
            .write_line(json::format(&result, true)?)?;

        return Ok(None);
    }

    let remove_count =
        result.cache.len() + result.plugins.len() + result.temp.len() + result.tools.len();

    if remove_count == 0 {
        session.console.render(element! {
            Notice(variant: Variant::Info) {
                StyledText(
                    content: format!("Clean complete but nothing was removed.\nNo artifacts were found older than {} days.", args.days)
                )
            }
        })?;
    } else {
        session.console.render(element! {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!("Clean complete, {} artifacts removed:", remove_count)
                )
                List {
                    #(if result.cache.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} cached items ({} bytes)",
                                        result.cache.len(),
                                        result.cache.iter().fold(0, |acc, x| acc + x.size)
                                    )
                                )
                            }
                        })
                    })
                    #(if result.plugins.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} downloaded plugins ({} bytes)",
                                        result.plugins.len(),
                                        result.plugins.iter().fold(0, |acc, x| acc + x.size)
                                    )
                                )
                            }
                        })
                    })
                    #(if result.temp.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} temporary files ({} bytes)",
                                        result.temp.len(),
                                        result.temp.iter().fold(0, |acc, x| acc + x.size)
                                    )
                                )
                            }
                        })
                    })
                    #(if result.tools.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} installed tool versions",
                                        result.tools.len(),
                                    )
                                )
                            }
                        })
                    })
                }
            }
        })?;
    }

    Ok(None)
}
