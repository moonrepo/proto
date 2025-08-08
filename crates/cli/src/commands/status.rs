use crate::error::ProtoCliError;
use crate::session::{LoadToolOptions, ProtoSession};
use clap::Args;
use iocraft::prelude::{Size, element};
use proto_core::{ToolContext, ToolSpec, VersionSpec};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Default, Serialize)]
struct StatusItem {
    is_installed: bool,
    config_source: Option<PathBuf>,
    config_version: ToolSpec,
    resolved_version: Option<VersionSpec>,
    product_dir: Option<PathBuf>,
}

#[derive(Args, Clone, Debug)]
pub struct StatusArgs {}

#[tracing::instrument(skip_all)]
pub async fn status(session: ProtoSession, _args: StatusArgs) -> AppResult {
    debug!("Determining active tools based on config...");

    let mut items = BTreeMap::<ToolContext, StatusItem>::default();
    let tools = session
        .load_all_tools_with_options(LoadToolOptions {
            detect_version: true,
            ..Default::default()
        })
        .await?;

    for mut tool in tools {
        let Some(spec) = tool.detected_version.clone() else {
            continue;
        };

        debug!(version = spec.to_string(), "Checking {}", tool.get_name());

        let item = items.entry(tool.context.clone()).or_default();

        // Resolve a version based on the configured spec, and ignore errors
        // as they indicate a version could not be resolved!
        if let Ok(version) = tool.resolve_version(&spec, false).await
            && !version.is_latest()
        {
            if tool.is_installed() {
                item.is_installed = true;
                item.product_dir = Some(tool.get_product_dir());
            }

            item.resolved_version = Some(version);
        }

        item.config_version = spec;
        item.config_source = tool.detected_source;
    }

    if items.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    debug!(
        tools = ?items.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Found tools with configured versions",
    );

    if session.should_print_json() {
        session
            .console
            .out
            .write_line(json::format(&items, true)?)?;

        return Ok(None);
    }

    let ctx_width = items.keys().fold(0, |acc, ctx| acc.max(ctx.as_str().len()));

    session.console.render(element! {
        Container {
            Table(
                headers: vec![
                    TableHeader::new("Tool", Size::Length((ctx_width + 3).max(10) as u32)),
                    TableHeader::new("Configured", Size::Length(12)),
                    TableHeader::new("Resolved", Size::Length(12)),
                    TableHeader::new("Installed", Size::Percent(30.0)),
                    TableHeader::new("Config", Size::Auto),
                ]
            ) {
                #(items.into_iter().enumerate().map(|(i, (ctx, item))| {
                    element! {
                        TableRow(row: i as i32) {
                            TableCol(col: 0) {
                                StyledText(
                                    content: ctx.to_string(),
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
