use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::{element, Size};
use miette::IntoDiagnostic;
use proto_core::{detect_version, Id, UnresolvedVersionSpec, VersionSpec, PROTO_PLUGIN_KEY};
use rustc_hash::FxHashSet;
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use tokio::task::JoinSet;
use tracing::debug;

#[derive(Debug, Default, Serialize)]
struct StatusItem {
    is_installed: bool,
    config_source: Option<PathBuf>,
    config_version: UnresolvedVersionSpec,
    resolved_version: Option<VersionSpec>,
    product_dir: Option<PathBuf>,
}

#[derive(Args, Clone, Debug)]
pub struct StatusArgs {}

async fn find_item_versions(session: &ProtoSession) -> miette::Result<BTreeMap<Id, StatusItem>> {
    let mut set = JoinSet::new();
    let mut items = BTreeMap::<Id, StatusItem>::default();

    // We need all tools so we can attempt to detect a version
    for tool in session.load_all_tools().await? {
        if tool.id == PROTO_PLUGIN_KEY {
            continue;
        }

        set.spawn(async move {
            if let Ok(detected) = detect_version(&tool, None).await {
                return Some((
                    tool.id.clone(),
                    detected,
                    env::var(format!("{}_DETECTED_FROM", tool.get_env_var_prefix()))
                        .ok()
                        .map(PathBuf::from),
                ));
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

    Ok(items)
}

async fn resolve_item_versions(
    session: &ProtoSession,
    items: &mut BTreeMap<Id, StatusItem>,
) -> miette::Result<()> {
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

    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn status(session: ProtoSession, _args: StatusArgs) -> AppResult {
    debug!("Determining active tools based on config...");

    let mut items = find_item_versions(&session).await?;

    if items.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    debug!(
        tools = ?items.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Found tools with configured versions",
    );

    resolve_item_versions(&session, &mut items).await?;

    if session.should_print_json() {
        session
            .console
            .out
            .write_line(json::format(&items, true)?)?;

        return Ok(None);
    }

    let id_width = items.keys().fold(0, |acc, id| acc.max(id.as_str().len()));

    session.console.render(element! {
        Container {
            Table(
                headers: vec![
                    TableHeader::new("Tool", Size::Length((id_width + 3).max(10) as u32)),
                    TableHeader::new("Configured", Size::Length(12)),
                    TableHeader::new("Resolved", Size::Length(12)),
                    TableHeader::new("Installed", Size::Percent(30.0)),
                    TableHeader::new("Config", Size::Auto),
                ]
            ) {
                #(items.into_iter().enumerate().map(|(i, (id, item))| {
                    element! {
                        TableRow(row: i as i32) {
                            TableCol(col: 0) {
                                StyledText(
                                    content: id.to_string(),
                                    style: Style::Id
                                )
                            }
                            TableCol(col: 1) {
                                StyledText(
                                    content: item.config_version.to_string(),
                                    style: Style::Invalid
                                )
                            }
                            TableCol(col: 2) {
                                #(if let Some(version) = item.resolved_version {
                                    element! {
                                        StyledText(
                                            content: version.to_string(),
                                            style: Style::Hash
                                        )
                                    }
                                } else {
                                    element! {
                                        StyledText(
                                            content: "N/A",
                                            style: Style::MutedLight
                                        )
                                    }
                                })
                            }
                            TableCol(col: 3) {
                                #(if let Some(dir) = item.product_dir {
                                    element! {
                                        StyledText(
                                            content: dir.to_string_lossy(),
                                            style: Style::Path
                                        )
                                    }
                                } else {
                                    element! {
                                        StyledText(
                                            content: "No",
                                            style: Style::MutedLight
                                        )
                                    }
                                })
                            }
                            TableCol(col: 4) {
                                 #(if let Some(src) = item.config_source {
                                    element! {
                                        StyledText(
                                            content: src.to_string_lossy(),
                                            style: Style::Path
                                        )
                                    }
                                } else {
                                    element! {
                                        StyledText(
                                            content: "N/A",
                                            style: Style::MutedLight
                                        )
                                    }
                                })
                            }
                        }
                    }
                }))
            }
        }
    })?;

    Ok(None)
}
