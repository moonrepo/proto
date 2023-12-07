use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use crate::printer::Printer;
use chrono::{DateTime, NaiveDateTime};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{Id, ProtoToolConfig, ToolManifest, UnresolvedVersionSpec};
use serde::Serialize;
use starbase::system;
use starbase_styles::color;
use starbase_utils::json;
use std::collections::{HashMap, HashSet};
use tokio::sync::Mutex;
use tracing::info;

#[derive(Serialize)]
pub struct ToolItem<'a> {
    manifest: ToolManifest,
    config: Option<&'a ProtoToolConfig>,
}

#[derive(Args, Clone, Debug)]
pub struct ListToolsArgs {
    #[arg(help = "ID of tools to list")]
    ids: Vec<Id>,

    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn list(args: ArgsRef<ListToolsArgs>, proto: ResourceRef<ProtoResource>) {
    if !args.json {
        info!("Loading tools...");
    }

    let tools = proto
        .load_tools_with_filters(HashSet::from_iter(&args.ids))
        .await?;

    let mut tools = tools
        .into_iter()
        .filter(|tool| !tool.manifest.installed_versions.is_empty())
        .collect::<Vec<_>>();

    tools.sort_by(|a, d| a.id.cmp(&d.id));

    if tools.is_empty() {
        return Err(ProtoCliError::NoInstalledTools.into());
    }

    let mut config = proto.env.load_config()?.to_owned();

    if args.json {
        let items = tools
            .into_iter()
            .map(|t| {
                let tool_config = config.tools.get(&t.id);

                (
                    t.id,
                    ToolItem {
                        manifest: t.manifest,
                        config: tool_config,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);

        return Ok(());
    }

    let printer = Mutex::new(Printer::new());
    let latest_version = UnresolvedVersionSpec::default();

    for tool in tools {
        let tool_config = config.tools.remove(&tool.id).unwrap_or_default();

        let mut versions = tool.load_version_resolver(&latest_version).await?;
        versions.aliases.extend(tool_config.aliases);

        let mut printer = printer.lock().await;

        printer.line();
        printer.header(&tool.id, &tool.metadata.name);

        printer.section(|p| {
            p.entry("Store", color::path(tool.get_inventory_dir()));

            p.entry_map(
                "Aliases",
                versions
                    .aliases
                    .iter()
                    .map(|(k, v)| (color::hash(v.to_string()), k))
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

                        if config
                            .versions
                            .get(&tool.id)
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

    printer.lock().await.flush();
}

fn create_datetime(millis: u128) -> Option<NaiveDateTime> {
    DateTime::from_timestamp((millis / 1000) as i64, ((millis % 1000) * 1_000_000) as u32)
        .map(|dt| dt.naive_local())
}
