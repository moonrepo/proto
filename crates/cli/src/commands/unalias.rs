use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use proto_core::{Id, ProtoConfig};
use starbase::AppResult;
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
        group = "pin",
        help = "Remove from the global ~/.proto/.prototools"
    )]
    global: bool,

    #[arg(long, group = "pin", help = "Location of .prototools to remove from")]
    from: Option<PinOption>,
}

#[tracing::instrument(skip_all)]
pub async fn unalias(session: ProtoSession, args: UnaliasArgs) -> AppResult {
    let tool = session.load_tool(&args.id).await?;
    let mut value = None;

    let config_path = ProtoConfig::update(
        tool.proto
            .get_config_dir(map_pin_type(args.global, args.from)),
        |config| {
            if let Some(tool_configs) = &mut config.tools {
                if let Some(tool_config) = tool_configs.get_mut(&tool.id) {
                    if let Some(aliases) = &mut tool_config.aliases {
                        value = aliases.remove(&args.alias);
                    }
                }
            }
        },
    )?;

    let Some(value) = value else {
        eprintln!(
            "Alias {} not found in config {}",
            color::id(&args.alias),
            color::path(config_path)
        );

        process::exit(1);
    };

    println!(
        "Removed alias {} ({}) from config {}",
        color::id(&args.alias),
        color::muted_light(value.to_string()),
        color::path(config_path)
    );

    Ok(())
}
