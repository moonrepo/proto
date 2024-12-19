use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{ProtoError, Tool, VersionSpec, PROTO_PLUGIN_KEY};
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

#[derive(Args, Clone, Debug, Default)]
pub struct CleanArgs {
    #[arg(
        long,
        help = "Clean tools and plugins older than the specified number of days"
    )]
    pub days: Option<u8>,

    #[arg(long, help = "Print the clean result in JSON format")]
    pub json: bool,

    #[arg(long, help = "Avoid and force confirm prompts", env = "PROTO_YES")]
    pub yes: bool,
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
    skip_prompts: bool,
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
            if is_older_than_days(now_millis, last_used, days) {
                debug!(
                    "Version {} hasn't been used in over {} days, removing",
                    color::hash(version.to_string()),
                    days
                );

                versions_to_clean.insert(version.to_owned());
            }
        }
    }

    if versions_to_clean.is_empty() {
        debug!("No versions to remove, continuing to next tool");

        return Ok(cleaned);
    }

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
    }

    if skip_prompts || confirmed {
        for version in versions_to_clean {
            cleaned.push(StaleTool {
                dir: inventory_dir.join(version.to_string()),
                id: tool.id.to_string(),
                version: version.clone(),
            });

            tool.set_version(version);
            tool.teardown().await?;
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

        let version = VersionSpec::parse(&dir_name).map_err(|error| ProtoError::VersionSpec {
            version: dir_name,
            error: Box::new(error),
        })?;

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
    skip_prompts: bool,
) -> miette::Result<CleanResult> {
    let days = args.days.unwrap_or(30) as u64;
    let now = SystemTime::now();
    let mut result = CleanResult::default();

    debug!("Cleaning installed tools...");

    for tool in session.load_tools().await? {
        result
            .tools
            .extend(clean_tool(session, tool.tool, now, days, skip_prompts).await?);
    }

    result
        .tools
        .extend(clean_proto_tool(session, now, days).await?);

    debug!("Cleaning downloaded plugins...");

    result.plugins = clean_dir(&session.env.store.plugins_dir, now, days).await?;

    debug!("Cleaning temporary directory...");

    result.temp = clean_dir(&session.env.store.temp_dir, now, days).await?;

    debug!("Cleaning cache directory...");

    result.cache = clean_dir(&session.env.store.cache_dir, now, days).await?;

    Ok(result)
}

#[instrument(skip_all)]
pub async fn clean(session: ProtoSession, args: CleanArgs) -> AppResult {
    let skip_prompts = session.skip_prompts(args.yes);
    let data = internal_clean(&session, &args, skip_prompts).await?;

    if args.json {
        session.console.out.write_line(json::format(&data, true)?)?;

        return Ok(None);
    }

    let remove_count = data.cache.len() + data.plugins.len() + data.temp.len() + data.tools.len();

    if remove_count == 0 {
        session.console.render(element! {
            Notice(variant: Variant::Info) {
                StyledText(
                    content: format!("Clean complete but nothing was removed.\nNo artifacts were found older than {} days.", args.days.unwrap_or(30))
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
                    #(if data.cache.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} cached items ({} bytes)",
                                        data.cache.len(),
                                        data.cache.iter().fold(0, |acc, x| acc + x.size)
                                    )
                                )
                            }
                        })
                    })
                    #(if data.plugins.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} downloaded plugins ({} bytes)",
                                        data.plugins.len(),
                                        data.plugins.iter().fold(0, |acc, x| acc + x.size)
                                    )
                                )
                            }
                        })
                    })
                    #(if data.temp.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} temporary files ({} bytes)",
                                        data.temp.len(),
                                        data.temp.iter().fold(0, |acc, x| acc + x.size)
                                    )
                                )
                            }
                        })
                    })
                    #(if data.tools.is_empty() {
                        None
                    } else {
                        Some(element! {
                            ListItem {
                                Text(
                                    content: format!(
                                        "{} installed tool versions",
                                        data.tools.len(),
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
