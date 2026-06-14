use crate::error::ProtoCliError;
use crate::session::{LoadToolOptions, ProtoSession};
use clap::Args;
use iocraft::prelude::Size;
use proto_core::flow::resolve::Resolver;
use proto_core::{ToolContext, ToolSpec, VersionSpec};
use serde::Serialize;
use starbase::AppResult;
use starbase_console::ui::*;
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

    session.console.table(
        vec![
            TableHeader::new("Tool", Size::Length((ctx_width + 3).max(10) as u32)),
            TableHeader::new("Configured", Size::Length(12)),
            TableHeader::new("Resolved", Size::Length(12)),
            TableHeader::new("Installed", Size::Percent(30.0)),
            TableHeader::new("Config", Size::Auto),
        ],
        items
            .into_iter()
            .map(|(ctx, item)| {
                vec![
                    format!("<id>{ctx}</id>"),
                    format!("<invalid>{}</invalid>", item.config_version),
                    if let Some(version) = item.resolved_version {
                        format!("<hash>{version}</hash>")
                    } else {
                        "<mutedlight>N/A</mutedlight>".into()
                    },
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
                ]
            })
            .collect(),
    )?;

    Ok(None)
}
