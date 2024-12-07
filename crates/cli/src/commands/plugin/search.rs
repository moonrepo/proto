use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::*;
use proto_core::registry::{PluginAuthor, PluginFormat};
use proto_core::PluginLocator;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;

#[derive(Args, Clone, Debug)]
pub struct SearchPluginArgs {
    #[arg(required = true, help = "Query to search available plugins")]
    query: String,

    #[arg(long, help = "Print the plugins in JSON format")]
    json: bool,
}

#[tracing::instrument(skip_all)]
pub async fn search(session: ProtoSession, args: SearchPluginArgs) -> AppResult {
    let mut registry = session.create_registry();
    let plugins = registry.load_external_plugins().await?;

    let query = &args.query;
    let queried_plugins = plugins
        .into_iter()
        .filter(|data| {
            data.id.as_str().contains(query)
                || data.name.contains(query)
                || data.description.contains(query)
        })
        .collect::<Vec<_>>();

    if args.json {
        session
            .console
            .out
            .write_line(json::format(&queried_plugins, true)?)?;

        return Ok(None);
    }

    if queried_plugins.is_empty() {
        session.console.render(element! {
            Notice(title: "No results".to_owned(), variant: Variant::Caution) {
                StyledText(
                    content: format!("Please try again, there are no plugins found in the registry for the query <shell>{query}</shell>"),
                )
            }
        })?;

        return Ok(Some(1));
    }

    session.console.render(element! {
        Container {
            Box(padding_top: 1, padding_left: 1, flex_direction: FlexDirection::Column) {
                StyledText(
                    content: format!("Search results for: <label>{query}</label>"),
                )
                StyledText(
                    content: "Learn more about plugins: <url>https://moonrepo.dev/docs/proto/plugins</url>"
                )
            }
            Table(
                headers: vec![
                    TableHeader::new("Plugin", Size::Percent(8.0)),
                    TableHeader::new("Author", Size::Percent(10.0)),
                    TableHeader::new("Format", Size::Percent(5.0)),
                    TableHeader::new("Description", Size::Percent(20.0)),
                    TableHeader::new("Locator", Size::Percent(57.0)),
                ]
            ) {
                #(queried_plugins.into_iter().enumerate().map(|(i, plugin)| {
                    element! {
                        TableRow(row: i as i32) {
                            TableCol(col: 0) {
                                StyledText(
                                    content: &plugin.name,
                                    style: Style::Id
                                )
                            }
                            TableCol(col: 1) {
                                Text(
                                    content: match &plugin.author {
                                        PluginAuthor::String(name) => name,
                                        PluginAuthor::Object(author) => &author.name,
                                    }
                                )
                            }
                            TableCol(col: 2) {
                                Text(
                                    content: match plugin.format {
                                        PluginFormat::Json => "JSON",
                                        PluginFormat::Toml => "TOML",
                                        PluginFormat::Wasm => "WASM",
                                        PluginFormat::Yaml => "YAML",
                                    }
                                )
                            }
                            TableCol(col: 3) {
                                Text(content: &plugin.description)
                            }
                            TableCol(col: 4) {
                                StyledText(
                                    content: plugin.locator.to_string(),
                                    style: match plugin.locator {
                                        PluginLocator::File(_) => Style::Path,
                                        PluginLocator::Url(_) => Style::Url,
                                        _ => Style::File,
                                    }
                                )
                            }
                        }
                    }
                }))
            }
            Box(padding_bottom: 1, padding_left: 1, flex_direction: FlexDirection::Row) {
                StyledText(
                    content: "Find a plugin above that you want to use? Enable it with: ",
                )
                StyledText(
                    content: "proto plugin add [id] [locator]",
                    style: Style::Shell
                )
            }
        }
    })?;

    Ok(None)
}
