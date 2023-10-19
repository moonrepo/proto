use crate::helpers::load_configured_tools;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{Id, PluginLocator};
use serde::Serialize;
use starbase::system;
use starbase_styles::color::{self, OwoStyle};
use starbase_utils::json;
use std::io::{BufWriter, Write};
use tracing::info;

#[derive(Serialize)]
pub struct PluginItem {
    id: Id,
    locator: PluginLocator,
    name: String,
    version: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct PluginsArgs {
    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn plugins(args: ArgsRef<PluginsArgs>) {
    if !args.json {
        info!("Loading plugins...");
    }

    let tools = load_configured_tools().await?;

    let mut items = tools
        .into_iter()
        .map(|tool| PluginItem {
            id: tool.id.to_owned(),
            locator: tool.locator.unwrap(),
            name: tool.metadata.name,
            version: tool.metadata.plugin_version,
        })
        .collect::<Vec<_>>();

    items.sort_by(|a, d| a.id.cmp(&d.id));

    if args.json {
        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);

        return Ok(());
    }

    let stdout = std::io::stdout();
    let mut buffer = BufWriter::new(stdout.lock());

    for item in items {
        writeln!(
            buffer,
            "{} {} {}",
            OwoStyle::new().bold().style(color::id(item.id)),
            color::muted("-"),
            color::muted_light(if let Some(version) = item.version {
                format!("{} v{version}", item.name)
            } else {
                item.name
            })
        )
        .unwrap();

        match item.locator {
            PluginLocator::SourceFile { path, .. } => {
                writeln!(
                    buffer,
                    "  Source: {}",
                    color::path(path.canonicalize().unwrap())
                )
                .unwrap();
            }
            PluginLocator::SourceUrl { url } => {
                writeln!(buffer, "  Source: {}", color::url(url)).unwrap();
            }
            PluginLocator::GitHub(github) => {
                writeln!(buffer, "  GitHub: {}", color::label(&github.repo_slug)).unwrap();

                writeln!(
                    buffer,
                    "  Tag: {}",
                    color::hash(github.tag.as_deref().unwrap_or("latest")),
                )
                .unwrap();
            }
            PluginLocator::Wapm(wapm) => {
                writeln!(buffer, "  Package: {}", color::label(&wapm.package_name)).unwrap();

                writeln!(
                    buffer,
                    "  Version: {}",
                    color::hash(wapm.version.as_deref().unwrap_or("latest")),
                )
                .unwrap();
            }
        };

        writeln!(buffer).unwrap();
    }

    buffer.flush().unwrap();
}
