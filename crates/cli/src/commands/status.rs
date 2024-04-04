use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use clap::Args;
use comfy_table::presets::NOTHING;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use miette::IntoDiagnostic;
use proto_core::{UnresolvedVersionSpec, VersionSpec};
use rustc_hash::FxHashSet;
use serde::Serialize;
use starbase::system;
use starbase_styles::color::Style;
use starbase_utils::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::spawn;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct StatusArgs {
    #[arg(long, help = "Include versions from global ~/.proto/.prototools")]
    include_global: bool,

    #[arg(long, help = "Print the active tools in JSON format")]
    json: bool,

    #[arg(long, help = "Only check versions in local .prototools")]
    only_local: bool,
}

#[derive(Serialize)]
pub struct StatusItem {
    is_installed: bool,
    config_source: PathBuf,
    config_version: UnresolvedVersionSpec,
    resolved_version: Option<VersionSpec>,
    product_dir: Option<PathBuf>,
}

#[system]
pub async fn status(args: ArgsRef<StatusArgs>, proto: ResourceRef<ProtoResource>) {
    let manager = proto.env.load_config_manager()?;
    let mut items = BTreeMap::default();

    debug!("Determining active tools based on config...");

    for file in manager.files.iter().rev() {
        if !file.exists
            || !args.include_global && file.global
            || args.only_local && !file.path.parent().is_some_and(|p| p == proto.env.cwd)
        {
            continue;
        }

        if let Some(file_versions) = &file.config.versions {
            for (tool_id, config_version) in file_versions {
                if items.contains_key(tool_id) {
                    continue;
                }

                items.insert(
                    tool_id.to_owned(),
                    StatusItem {
                        is_installed: false,
                        config_source: file.path.to_owned(),
                        config_version: config_version.to_owned(),
                        resolved_version: None,
                        product_dir: None,
                    },
                );
            }
        };
    }

    if items.is_empty() {
        return Err(ProtoCliError::NoConfiguredTools.into());
    }

    debug!(
        tools = ?items.keys().map(|id| id.as_str()).collect::<Vec<_>>(),
        "Found tools with configured versions, loading them",
    );

    let tools = proto
        .load_tools_with_filters(FxHashSet::from_iter(items.keys()))
        .await?;
    let mut futures = vec![];

    for mut tool in tools {
        let Some(item) = items.get(&tool.id) else {
            continue;
        };

        let config_version = item.config_version.to_owned();

        futures.push(spawn(async move {
            debug!("Checking {}", tool.get_name());

            let mut resolved_version = None;
            let mut product_dir = None;

            // Resolve a version based on the configured spec, and ignore errors
            // as they indicate a version could not be resolved!
            if tool.resolve_version(&config_version, false).await.is_ok() {
                let version = tool.get_resolved_version();

                // Determine the install status
                if !version.is_latest() {
                    if tool.is_installed() {
                        product_dir = Some(tool.get_product_dir());
                    }

                    resolved_version = Some(version);
                }
            }

            (tool.id, resolved_version, product_dir)
        }));
    }

    for future in futures {
        let (id, resolved_version, product_dir) = future.await.into_diagnostic()?;

        if let Some(item) = items.get_mut(&id) {
            item.is_installed = product_dir.is_some();
            item.resolved_version = resolved_version;
            item.product_dir = product_dir;
        };
    }

    // Dump all the data as JSON
    if args.json {
        println!("{}", json::format(&items, true)?);

        return Ok(());
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
}
