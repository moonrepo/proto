use crate::helpers::ProtoResource;
use crate::printer::Printer;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{Id, PluginLocator};
use serde::Serialize;
use starbase::system;
use starbase_utils::json;
use tracing::info;

#[derive(Serialize)]
pub struct PluginItem {
    id: Id,
    locator: PluginLocator,
    name: String,
    version: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub struct ListToolPluginsArgs {
    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn list_plugins(args: ArgsRef<ListToolPluginsArgs>, proto: ResourceRef<ProtoResource>) {
    if !args.json {
        info!("Loading plugins...");
    }

    let tools = proto.load_tools().await?;

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

    let mut printer = Printer::new();

    for item in items {
        printer.line();
        printer.header(
            item.id,
            if let Some(version) = item.version {
                format!("{} v{version}", item.name)
            } else {
                item.name
            },
        );

        printer.section(|p| {
            p.locator(item.locator);
            Ok(())
        })?;
    }

    printer.flush();
}
