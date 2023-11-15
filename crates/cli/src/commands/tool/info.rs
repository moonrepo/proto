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
pub struct ListPluginsArgs {
    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn info_plugin(args: ArgsRef<ListPluginsArgs>) {
    if !args.json {
        info!("Loading plugins...");
    }

    let tools = load_configured_tools().await?;
}
