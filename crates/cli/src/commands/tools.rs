use crate::helpers::load_configured_tools_with_filters;
use chrono::{DateTime, NaiveDateTime};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::Id;
use starbase::system;
use starbase_styles::color::{self, OwoStyle};
use starbase_utils::json;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use std::process;
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct ToolsArgs {
    #[arg(help = "IDs of tool to list")]
    ids: Vec<Id>,

    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn tools(args: ArgsRef<ToolsArgs>) {
    if !args.json {
        info!("Loading tools...");
    }

    let tools = load_configured_tools_with_filters(HashSet::from_iter(&args.ids)).await?;

    let mut tools = tools
        .into_iter()
        .filter(|tool| !tool.manifest.installed_versions.is_empty())
        .collect::<Vec<_>>();

    tools.sort_by(|a, d| a.id.cmp(&d.id));

    if tools.is_empty() {
        eprintln!("No installed tools");
        process::exit(1);
    }

    if args.json {
        let items = tools
            .into_iter()
            .map(|t| (t.id, t.manifest))
            .collect::<HashMap<_, _>>();

        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);

        return Ok(());
    }

    let stdout = std::io::stdout();
    let mut buffer = BufWriter::new(stdout.lock());

    for tool in tools {
        writeln!(
            buffer,
            "{} {} {}",
            OwoStyle::new().bold().style(color::id(&tool.id)),
            color::muted("-"),
            color::muted_light(&tool.metadata.name),
        )
        .unwrap();

        writeln!(buffer, "  Store: {}", color::path(tool.get_inventory_dir())).unwrap();

        if !tool.manifest.aliases.is_empty() {
            writeln!(buffer, "  Aliases:").unwrap();

            for (alias, version) in &tool.manifest.aliases {
                writeln!(
                    buffer,
                    "    {} {} {}",
                    color::hash(version.to_string()),
                    color::muted("="),
                    color::label(alias),
                )
                .unwrap();
            }
        }

        if !tool.manifest.installed_versions.is_empty() {
            writeln!(buffer, "  Versions:").unwrap();

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
                    writeln!(buffer, "    {}", color::hash(version.to_string())).unwrap();
                } else {
                    writeln!(
                        buffer,
                        "    {} {} {}",
                        if is_default {
                            color::symbol(version.to_string())
                        } else {
                            color::hash(version.to_string())
                        },
                        color::muted("-"),
                        color::muted_light(comments.join(", "))
                    )
                    .unwrap();
                }
            }
        }

        writeln!(buffer).unwrap();
    }

    buffer.flush().unwrap();
}

fn create_datetime(millis: u128) -> Option<NaiveDateTime> {
    DateTime::from_timestamp((millis / 1000) as i64, ((millis % 1000) * 1_000_000) as u32)
        .map(|dt| dt.naive_local())
}
