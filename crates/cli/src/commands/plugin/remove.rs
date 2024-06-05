use crate::error::ProtoCliError;
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
        help = "Remove from the global ~/.proto/.prototools instead of local ./.prototools"
    )]
    global: bool,
}

#[tracing::instrument(skip_all)]
pub async fn remove(session: ProtoSession, args: RemovePluginArgs) -> AppResult {
    if !args.global {
        let config_path = session.env.cwd.join(PROTO_CONFIG_NAME);

        if !config_path.exists() {
            return Err(ProtoCliError::MissingToolsConfigInCwd { path: config_path }.into());
        }
    }

    let config_path = ProtoConfig::update(session.env.get_config_dir(args.global), |config| {
        if let Some(plugins) = &mut config.plugins {
            plugins.remove(&args.id);
        }
    })?;

    println!(
        "Removed plugin {} from config {}",
        color::id(&args.id),
        color::path(config_path)
    );

    Ok(())
}
