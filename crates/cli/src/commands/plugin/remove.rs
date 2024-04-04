use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::{Id, ProtoConfig, PROTO_CONFIG_NAME};
use starbase::system;
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

#[system]
pub async fn remove(args: ArgsRef<RemovePluginArgs>, proto: ResourceRef<ProtoResource>) {
    if !args.global {
        let config_path = proto.env.cwd.join(PROTO_CONFIG_NAME);

        if !config_path.exists() {
            return Err(ProtoCliError::MissingToolsConfigInCwd { path: config_path }.into());
        }
    }

    let config_path = ProtoConfig::update(proto.env.get_config_dir(args.global), |config| {
        if let Some(plugins) = &mut config.plugins {
            plugins.remove(&args.id);
        }
    })?;

    println!(
        "Removed plugin {} from config {}",
        color::id(&args.id),
        color::path(config_path)
    );
}
