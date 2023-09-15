use crate::helpers::load_configured_tools;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{Id, PluginLocator};
use serde::Serialize;
use starbase::system;
use starbase_styles::color::{self, OwoStyle};
use starbase_utils::json;
use std::collections::HashSet;
use tracing::info;

fn render_entry<V: AsRef<str>>(label: &str, value: V) {
    println!(
        "  {} {}",
        color::muted_light(format!("{label}:")),
        value.as_ref()
    );
}

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

    let mut items = vec![];

    load_configured_tools(HashSet::new(), |tool, locator| {
        items.push(PluginItem {
            id: tool.id.to_owned(),
            locator,
            name: tool.metadata.name,
            version: tool.metadata.plugin_version,
        });
    })
    .await?;

    items.sort_by(|a, d| a.id.cmp(&d.id));

    if args.json {
        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);

        return Ok(());
    }

    for item in items {
        println!(
            "{} {} {} {}",
            OwoStyle::new().bold().style(color::id(item.id)),
            color::muted("-"),
            item.name,
            color::muted_light(if let Some(version) = item.version {
                format!("v{version}")
            } else {
                "".into()
            })
        );

        match item.locator {
            PluginLocator::SourceFile { path, .. } => {
                render_entry("Source", color::path(path.canonicalize().unwrap()));
            }
            PluginLocator::SourceUrl { url } => {
                render_entry("Source", color::url(url));
            }
            PluginLocator::GitHub(github) => {
                render_entry("GitHub", color::label(&github.repo_slug));
                render_entry("Tag", github.tag.as_deref().unwrap_or("latest"));
            }
            PluginLocator::Wapm(wapm) => {
                render_entry("Package", color::label(&wapm.package_name));
                render_entry("Version", wapm.version.as_deref().unwrap_or("latest"));
            }
        };

        println!();
    }
}
