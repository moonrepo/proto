use crate::error::ProtoCliError;
use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use proto_core::{Id, ProtoConfig, PROTO_CONFIG_NAME};
use starbase::AppResult;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct RemovePluginArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

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
pub async fn remove(session: ProtoSession, args: RemovePluginArgs) -> AppResult {
    let config_dir = session
        .env
        .get_config_dir(map_pin_type(args.global, args.from));
    let config_path = config_dir.join(PROTO_CONFIG_NAME);

    if !config_path.exists() {
        return Err(ProtoCliError::MissingToolsConfigInCwd { path: config_path }.into());
    }

    let config_path = ProtoConfig::update(config_dir, |config| {
        if let Some(plugins) = &mut config.plugins {
            plugins.remove(&args.id);
        }
    })?;

    println!(
        "Removed plugin {} from config {}",
        color::id(&args.id),
        color::path(config_path)
    );

    Ok(None)
}
