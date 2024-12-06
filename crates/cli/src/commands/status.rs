use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use comfy_table::presets::NOTHING;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use miette::IntoDiagnostic;
use proto_core::{Id, UnresolvedVersionSpec, VersionSpec, PROTO_PLUGIN_KEY};
use rustc_hash::FxHashSet;
use serde::Serialize;
use starbase::AppResult;
use starbase_styles::color::Style;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct StatusArgs {
    #[arg(long, help = "Print the active tools in JSON format")]
    json: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct StatusItem {
    is_installed: bool,
    config_source: PathBuf,
    config_version: UnresolvedVersionSpec,
    resolved_version: Option<VersionSpec>,
    product_dir: Option<PathBuf>,
}

fn find_versions_in_configs(
    session: &ProtoSession,
    items: &mut BTreeMap<Id, StatusItem>,
) -> AppResult {
    let env = &session.env;
    let manager = env.load_config_manager()?;

    for file in manager.files.iter().rev() {
        if !file.exists
            || !env.config_mode.includes_global() && file.global
            || env.config_mode.only_local()
                && file.path.parent().is_none_or(|p| p != session.env.cwd)
        {
            continue;
        }

        if let Some(file_versions) = &file.config.versions {
            for (tool_id, config_version) in file_versions {
                if tool_id == PROTO_PLUGIN_KEY {
                    continue;
                }

                items.insert(
                    tool_id.to_owned(),
                    StatusItem {
                        config_source: file.path.to_owned(),
                        config_version: config_version.to_owned(),
                        ..Default::default()
                    },
                );
            }
        };
    }

    Ok(None)
}

async fn find_versions_from_ecosystem(
    session: &ProtoSession,
    items: &mut BTreeMap<Id, StatusItem>,
) -> AppResult {
    let mut set = JoinSet::new();

    for tool in session.load_tools().await? {
        let env = Arc::clone(&session.env);

        set.spawn(async move {
            if let Ok(Some(detected)) = tool.detect_version_from(&env.cwd).await {
                return Some((tool.id.clone(), detected.0, detected.1));
            }

            None
        });
    }

    while let Some(result) = set.join_next().await {
        if let Some((id, version, source)) = result.into_diagnostic()? {
            let item = items.entry(id).or_default();
            item.config_version = version;
            item.config_source = source;
        }
    }

    Ok(None)
}

async fn resolve_item_versions(
    session: &ProtoSession,
    items: &mut BTreeMap<Id, StatusItem>,
) -> AppResult {
    let mut set = JoinSet::new();

    for mut tool in session
        .load_tools_with_filters(FxHashSet::from_iter(items.keys()))
        .await?
    {
        let Some(item) = items.get(&tool.id) else {
            continue;
        };

        let config_version = item.config_version.to_owned();

        set.spawn(async move {
            debug!("Checking {}", tool.get_name());

            let mut resolved_version = None;
            let mut product_dir = None;

            // Resolve a version based on the configured spec, and ignore errors
            // as they indicate a version could not be resolved!
            if let Ok(version) = tool.resolve_version(&config_version, false).await {
                // Determine the install status
                if !version.is_latest() {
                    if tool.is_installed() {
                        product_dir = Some(tool.get_product_dir());
                    }

                    resolved_version = Some(version);
                }
            }

            (tool.id.clone(), resolved_version, product_dir)
        });
    }

    while let Some(result) = set.join_next().await {
        let (id, resolved_version, product_dir) = result.into_diagnostic()?;

        if let Some(item) = items.get_mut(&id) {
            item.is_installed = product_dir.is_some();
            item.resolved_version = resolved_version;
            item.product_dir = product_dir;
        };
    }

    Ok(None)
}

#[tracing::instrument(skip_all)]
pub async fn status(session: ProtoSession, args: StatusArgs) -> AppResult {
    debug!("Determining active tools based on config...");

    let mut items = BTreeMap::default();

    find_versions_in_configs(&session, &mut items)?;
    find_versions_from_ecosystem(&session, &mut items).await?;

    if items.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    debug!(
        tools = ?items.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Found tools with configured versions",
    );

    resolve_item_versions(&session, &mut items).await?;

    // Dump all the data as JSON
    if args.json {
        println!("{}", json::format(&items, true)?);

        return Ok(None);
    }

    // Print all the data in a table
    let mut table = Table::new();
    table.load_preset(NOTHING);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec![
        Cell::new("Tool").add_attribute(Attribute::Bold),
        Cell::new("Configured").add_attribute(Attribute::Bold),
        Cell::new("Resolved").add_attribute(Attribute::Bold),
        Cell::new("Installed").add_attribute(Attribute::Bold),
        Cell::new("Config").add_attribute(Attribute::Bold),
    ]);

    for (id, item) in items {
        table.add_row(vec![
            Cell::new(id).fg(Color::AnsiValue(Style::Id.color() as u8)),
            Cell::new(&item.config_version),
            if let Some(version) = item.resolved_version {
                Cell::new(version.to_string()).fg(Color::AnsiValue(Style::Success.color() as u8))
            } else {
                Cell::new("Invalid").fg(Color::AnsiValue(Style::MutedLight.color() as u8))
            },
            if let Some(dir) = item.product_dir {
                Cell::new(dir.to_string_lossy()).fg(Color::AnsiValue(Style::Path.color() as u8))
            } else {
                Cell::new("No").fg(Color::AnsiValue(Style::MutedLight.color() as u8))
            },
            Cell::new(item.config_source.to_string_lossy())
                .fg(Color::AnsiValue(Style::Path.color() as u8)),
        ]);
    }

    println!("\n{table}\n");

    Ok(None)
}
