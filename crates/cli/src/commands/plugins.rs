use crate::tools::create_tool_from_plugin;
use miette::IntoDiagnostic;
use proto_core::{Id, PluginLocator, ProtoEnvironment, ToolsConfig, UserConfig};
use serde::Serialize;
use starbase::SystemResult;
use starbase_styles::color;
use starbase_utils::json;
use std::collections::HashMap;
use tracing::debug;

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

pub async fn plugins(json: bool) -> SystemResult {
    let proto = ProtoEnvironment::new()?;
    let user_config = UserConfig::load()?;

    let mut tools_config = ToolsConfig::load_upwards()?;
    tools_config.inherit_builtin_plugins();

    let mut plugins = HashMap::new();
    plugins.extend(user_config.plugins);
    plugins.extend(tools_config.plugins);

    debug!("Loading plugins");

    let mut items = vec![];

    for (id, locator) in plugins {
        let tool = create_tool_from_plugin(&id, &proto, &locator).await?;

        items.push(PluginItem {
            id,
            locator,
            name: tool.metadata.name,
            version: tool.metadata.plugin_version,
        });
    }

    items.sort_by(|a, d| a.id.cmp(&d.id));

    if json {
        println!("{}", json::to_string_pretty(&items).into_diagnostic()?);

        return Ok(());
    }

    for item in items {
        println!(
            "{} {} {} {}",
            color::id(item.id),
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

    Ok(())
}
