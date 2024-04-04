use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::{Id, ProtoConfig};
use starbase::system;
use starbase_styles::color;
use std::process;

#[derive(Args, Clone, Debug)]
pub struct UnaliasArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Alias name")]
    alias: String,

    #[arg(
        long,
        help = "Remove from the global ~/.proto/.prototools instead of local ./.prototools"
    )]
    global: bool,
}

#[system]
pub async fn unalias(args: ArgsRef<UnaliasArgs>, proto: ResourceRef<ProtoResource>) {
    let tool = proto.load_tool(&args.id).await?;
    let mut value = None;

    let config_path = ProtoConfig::update(tool.proto.get_config_dir(args.global), |config| {
        if let Some(tool_configs) = &mut config.tools {
            if let Some(tool_config) = tool_configs.get_mut(&tool.id) {
                if let Some(aliases) = &mut tool_config.aliases {
                    value = aliases.remove(&args.alias);
                }
            }
        }
    })?;

    if value.is_none() {
        eprintln!(
            "Alias {} not found in config {}",
            color::id(&args.alias),
            color::path(config_path)
        );

        process::exit(1);
    }

    println!(
        "Removed alias {} ({}) from config {}",
        color::id(&args.alias),
        color::muted_light(value.unwrap().to_string()),
        color::path(config_path)
    );
}
