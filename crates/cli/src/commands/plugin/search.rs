use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::{FlexDirection, Size, Text, View, element};
use proto_core::PluginLocator;
use proto_core::registry::PluginFormat;
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_utils::json;

#[derive(Args, Clone, Debug)]
pub struct SearchPluginArgs {
    #[arg(required = true, help = "Query to search available plugins")]
    query: String,
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

    if session.should_print_json() {
        session
            .console
            .out
            .write_line(json::format(&queried_plugins, true)?)?;

        return Ok(None);
    }

    if queried_plugins.is_empty() {
        session.console.render_err(element! {
            Notice(title: "No results".to_owned(), variant: Variant::Caution) {
                StyledText(
                    content: format!("Please try again, there are no plugins found in the registry for the query <shell>{query}</shell>"),
                )
            }
        })?;

        return Ok(Some(1));
    }

    let (name_width, author_width) = queried_plugins.iter().fold((0, 0), |acc, plugin| {
        (
            acc.0.max(plugin.name.len()),
            acc.1.max(plugin.author.get_name().len()),
        )
    });

    session.console.render(element! {
        Container {
            View(padding_top: 1, padding_left: 1, flex_direction: FlexDirection::Column) {
                StyledText(
                    content: format!("Search results for: <label>{query}</label>"),
                )
                StyledText(
                    content: "Learn more about plugins: <url>https://moonrepo.dev/docs/proto/plugins</url>"
                )
            }
            Table(
                headers: vec![
                    TableHeader::new("Plugin", Size::Length(name_width.max(6) as u32)),
                    TableHeader::new("Author", Size::Length(author_width.max(6) as u32)),
                    TableHeader::new("Format", Size::Length(6)),
                    TableHeader::new("Description", Size::Percent(30.0)),
                    TableHeader::new("Locator", Size::Auto),
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
                                    content: plugin.author.get_name()
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
            View(padding_bottom: 1, padding_left: 1, flex_direction: FlexDirection::Row) {
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
