use crate::printer::Printer;
use crate::session::ProtoSession;
use clap::Args;
use comfy_table::presets::NOTHING;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use proto_core::registry::{PluginAuthor, PluginFormat};
use starbase::AppResult;
use starbase_styles::color::{self, Style};
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

    // Dump all the data as JSON
    if args.json {
        println!("{}", json::format(&queried_plugins, true)?);

        return Ok(None);
    }

    if queried_plugins.is_empty() {
        eprintln!("No plugins available for query \"{query}\"");

        return Ok(Some(1));
    }

    // Print all the data in a table
    let mut printer = Printer::new();

    printer.named_section("Plugins", |p| {
        p.write(format!("Available for query: {}\n", color::property(query)));

        let mut table = Table::new();
        table.load_preset(NOTHING);
        table.set_content_arrangement(ContentArrangement::Dynamic);

        table.set_header(vec![
            Cell::new("Plugin").add_attribute(Attribute::Bold),
            Cell::new("Author").add_attribute(Attribute::Bold),
            Cell::new("Format").add_attribute(Attribute::Bold),
            Cell::new("Description").add_attribute(Attribute::Bold),
            Cell::new("Locator").add_attribute(Attribute::Bold),
        ]);

        for plugin in queried_plugins {
            table.add_row(vec![
                Cell::new(&plugin.name).fg(Color::AnsiValue(Style::Id.color() as u8)),
                Cell::new(match &plugin.author {
                    PluginAuthor::String(name) => name,
                    PluginAuthor::Object(author) => &author.name,
                }),
                Cell::new(match plugin.format {
                    PluginFormat::Json => "JSON",
                    PluginFormat::Toml => "TOML",
                    PluginFormat::Wasm => "WASM",
                    PluginFormat::Yaml => "YAML",
                }),
                Cell::new(&plugin.description),
                Cell::new(plugin.locator.to_string())
                    .fg(Color::AnsiValue(Style::Path.color() as u8)),
            ]);
        }

        p.write(format!("{table}"));

        Ok(())
    })?;

    printer.named_section("Usage", |p| {
        p.write("Find a plugin above that you want to use? Enable it with the command below.");
        p.write(format!(
            "Learn more about plugins: {}",
            color::url("https://moonrepo.dev/docs/proto/plugins")
        ));
        p.line();
        p.write(color::shell(" proto plugin add <id> <locator>"));

        Ok(())
    })?;

    printer.flush();

    Ok(None)
}
