use crate::helpers::load_configured_tools;
use chrono::{DateTime, NaiveDateTime};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::Id;
use starbase::system;
use starbase_styles::color::{self, OwoStyle};
use starbase_utils::json;
use std::collections::{HashMap, HashSet};
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct ToolsArgs {
    #[arg(help = "IDs of tool to list")]
    id: Vec<Id>,

    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn tools(args: ArgsRef<ToolsArgs>) {
    if !args.json {
        info!("Loading tools...");
    }

    let mut tools = vec![];

    load_configured_tools(HashSet::from_iter(&args.id), |tool, _| {
        if !tool.manifest.installed_versions.is_empty() {
            tools.push(tool);
        }
    })
    .await?;

    tools.sort_by(|a, d| a.id.cmp(&d.id));

    if args.json {
        let items = tools
            .into_iter()
            .map(|t| (t.id, t.manifest))
            .collect::<HashMap<_, _>>();

        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);

        return Ok(());
    }

    for tool in tools {
        println!(
            "{} {} {}",
            OwoStyle::new().bold().style(color::id(&tool.id)),
            color::muted("-"),
            tool.metadata.name,
        );

        println!("  Store: {}", color::path(tool.get_inventory_dir()));

        if !tool.manifest.aliases.is_empty() {
            println!("  Aliases:");

            for (alias, version) in &tool.manifest.aliases {
                println!(
                    "    {} {} {}",
                    color::hash(version.to_string()),
                    color::muted("="),
                    color::label(alias),
                );
            }
        }

        if !tool.manifest.installed_versions.is_empty() {
            println!("  Versions:");

            let mut versions = tool.manifest.installed_versions.iter().collect::<Vec<_>>();
            versions.sort();

            for version in versions {
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

                if tool
                    .manifest
                    .default_version
                    .as_ref()
                    .is_some_and(|dv| dv == &version.to_unresolved_spec())
                {
                    comments.push("default version".into());
                    is_default = true;
                }

                if comments.is_empty() {
                    println!("    {}", color::hash(version.to_string()));
                } else {
                    println!(
                        "    {} {} {}",
                        if is_default {
                            color::symbol(version.to_string())
                        } else {
                            color::hash(version.to_string())
                        },
                        color::muted("-"),
                        color::muted_light(comments.join(", "))
                    );
                }
            }
        }

        println!();
    }
}

fn create_datetime(millis: u128) -> Option<NaiveDateTime> {
    DateTime::from_timestamp((millis / 1000) as i64, ((millis % 1000) * 1_000_000) as u32)
        .map(|dt| dt.naive_local())
}
