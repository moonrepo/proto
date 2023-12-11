use crate::helpers::ProtoResource;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{Id, PluginLocator, ProtoConfig, ProtoConfigFile};
use serde::Serialize;
use starbase::system;
use starbase_utils::json;
use tracing::info;

#[derive(Serialize)]
pub struct DebugConfigResult {
    config: ProtoConfig,
    files: Vec<ProtoConfigFile>,
}

#[derive(Args, Clone, Debug)]
pub struct DebugConfigArgs {
    #[arg(long, help = "Print the list in JSON format")]
    json: bool,
}

#[system]
pub async fn config(args: ArgsRef<DebugConfigArgs>, proto: ResourceRef<ProtoResource>) {
    let manager = proto.env.load_config_manager()?;

    if args.json {
        let result = DebugConfigResult {};

        println!("{}", json::to_string_pretty(&result).into_diagnostic()?);

        return Ok(());
    }
}
