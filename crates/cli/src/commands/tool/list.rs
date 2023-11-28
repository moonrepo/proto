use crate::error::ProtoCliError;
use crate::helpers::load_configured_tools_with_filters;
use crate::printer::Printer;
use chrono::{DateTime, NaiveDateTime};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{Id, ToolManifest, UserConfig, UserToolConfig};
use serde::Serialize;
use starbase::system;
use starbase_styles::color;
use starbase_utils::json;
use std::collections::{HashMap, HashSet};
use tracing::info;

#[derive(Serialize)]
pub struct ToolItem {
    manifest: ToolManifest,
    user_config: Option<UserToolConfig>,
}

#[derive(Args, Clone, Debug)]
pub struct ListToolsArgs {
    #[arg(help = "ID of tools to list")]
    ids: Vec<Id>,

    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn list(args: ArgsRef<ListToolsArgs>) {
    if !args.json {
        info!("Loading tools...");
    }

    let tools = load_configured_tools_with_filters(HashSet::from_iter(&args.ids)).await?;
    let mut user_config = UserConfig::load()?;

    let mut tools = tools
        .into_iter()
        .filter(|tool| !tool.manifest.installed_versions.is_empty())
        .collect::<Vec<_>>();

    tools.sort_by(|a, d| a.id.cmp(&d.id));

    if tools.is_empty() {
        return Err(ProtoCliError::NoInstalledTools.into());
    }

    if args.json {
        let items = tools
            .into_iter()
            .map(|t| {
                let user_config = user_config.tools.remove(&t.id);

                (
                    t.id,
                    ToolItem {
                        manifest: t.manifest,
                        user_config,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);

        return Ok(());
    }

    let mut printer = Printer::new();

    for tool in tools {
        let user_tool_config = user_config.tools.remove(&tool.id).unwrap_or_default();

        printer.line();
        printer.header(&tool.id, &tool.metadata.name);

        printer.section(|p| {
            p.entry("Store", color::path(tool.get_inventory_dir()));

            p.entry_map(
                "Aliases",
                user_tool_config
                    .aliases
                    .iter()
                    .map(|(k, v)| (color::hash(v.to_string()), color::label(k)))
                    .collect::<Vec<_>>(),
                None,
            );

            let mut versions = tool.manifest.installed_versions.iter().collect::<Vec<_>>();
            versions.sort();

            p.entry_map(
                "Versions",
                versions
                    .iter()
                    .map(|version| {
                        let mut comments = vec![];
                        let mut is_default = false;

                        if let Some(meta) = &tool.manifest.versions.get(version) {
                            if let Some(at) = create_datetime(meta.installed_at) {
                                comments.push(format!("installed {}", at.format("%x")));
                            }

                            if let Some(last_used) = &meta.last_used_at {
                                if let Some(at) = create_datetime(*last_used) {
                                    comments.push(format!("last used {}", at.format("%x")));
                                }
                            }
                        }

                        if user_tool_config
                            .default_version
                            .as_ref()
                            .is_some_and(|dv| *dv == version.to_unresolved_spec())
                        {
                            comments.push("default version".into());
                            is_default = true;
                        }

                        (
                            if is_default {
                                color::invalid(version.to_string())
                            } else {
                                color::hash(version.to_string())
                            },
                            color::muted_light(comments.join(", ")),
                        )
                    })
                    .collect::<Vec<_>>(),
                None,
            );

            Ok(())
        })?;
    }

    printer.flush();
}

fn create_datetime(millis: u128) -> Option<NaiveDateTime> {
    DateTime::from_timestamp((millis / 1000) as i64, ((millis % 1000) * 1_000_000) as u32)
        .map(|dt| dt.naive_local())
}
