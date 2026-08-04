use crate::error::ProtoCliError;
use crate::session::{LoadToolOptions, ProtoSession, SessionResult};
use clap::Args;
use iocraft::prelude::Size;
use proto_core::flow::resolve::Resolver;
use proto_core::{ToolContext, ToolSpec, VersionSpec};
use serde::Serialize;
use starbase_console::ui::*;
use starbase_styles::encode_style_tags;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::{debug, instrument};

#[derive(Debug, Default, Serialize)]
struct StatusItem {
    is_installed: bool,
    config_source: Option<PathBuf>,
    config_version: ToolSpec,
    locked_version: Option<VersionSpec>,
    resolved_version: Option<VersionSpec>,
    product_dir: Option<PathBuf>,
}

#[derive(Args, Clone, Debug)]
pub struct StatusArgs {}

#[instrument(skip(session))]
pub async fn status(session: ProtoSession, _args: StatusArgs) -> SessionResult {
    debug!("Determining active tools based on config...");

    let mut items = BTreeMap::<ToolContext, StatusItem>::default();
    let tools = session
        .load_all_tools_with_options(LoadToolOptions {
            detect_version: true,
            ..Default::default()
        })
        .await?;

    for tool in tools {
        let Some(mut spec) = tool.detected_version.clone() else {
            continue;
        };

        debug!(version = spec.to_string(), "Checking {}", tool.get_name());

        let item = items.entry(tool.context.clone()).or_default();

        // Resolve a version based on the configured spec, and ignore errors
        // as they indicate a version could not be resolved!
        if let Ok(version) = Resolver::resolve(&tool, &mut spec, false).await
            && !version.is_latest()
        {
            if tool.is_installed(&spec) {
                item.is_installed = true;
                item.product_dir = Some(tool.get_product_dir(&spec));
            }

            item.resolved_version = Some(version);
        }

        // If the version was inherited from a lockfile record
        // during resolve, then the version is locked
        item.locked_version = spec
            .version_locked
            .as_ref()
            .and_then(|record| record.version.clone());
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

    if session.is_json_format() {
        session.console.write_json_for_format(items)?;

        return Ok(None);
    }

    let ctx_width = items.keys().fold(0, |acc, ctx| acc.max(ctx.as_str().len()));

    // Only show the locked column if a tool is using a lockfile
    let show_locked = items.values().any(|item| item.locked_version.is_some());

    let mut headers = vec![
        TableHeader::new("Tool", Size::Length((ctx_width + 3).max(10) as u32)),
        TableHeader::new("Configured", Size::Length(12)),
        TableHeader::new("Resolved", Size::Length(12)),
    ];

    if show_locked {
        headers.push(TableHeader::new("Locked", Size::Length(12)));
    }

    headers.extend([
        TableHeader::new("Installed", Size::Percent(30.0)),
        TableHeader::new("Config", Size::Auto),
    ]);

    session.console.table(
        headers,
        items
            .into_iter()
            .map(|(ctx, item)| {
                let mut row = vec![
                    format!("<id>{ctx}</id>"),
                    format!(
                        "<invalid>{}</invalid>",
                        encode_style_tags(item.config_version.to_string())
                    ),
                    if let Some(version) = item.resolved_version {
                        format!("<hash>{version}</hash>")
                    } else {
                        "<mutedlight>N/A</mutedlight>".into()
                    },
                ];

                if show_locked {
                    row.push(if let Some(version) = item.locked_version {
                        format!("<hash>{version}</hash>")
                    } else {
                        "<mutedlight>N/A</mutedlight>".into()
                    });
                }

                row.extend([
                    if let Some(dir) = item.product_dir {
                        format!("<path>{}</path>", dir.to_string_lossy())
                    } else {
                        "<mutedlight>No</mutedlight>".into()
                    },
                    if let Some(src) = item.config_source {
                        format!("<path>{}</path>", src.to_string_lossy())
                    } else {
                        "<mutedlight>N/A</mutedlight>".into()
                    },
                ]);

                row
            })
            .collect(),
    )?;

    Ok(None)
}
