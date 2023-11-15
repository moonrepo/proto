use crate::error::ProtoCliError;
use clap::Args;
use proto_core::{Id, ToolsConfig, UserConfig, TOOLS_CONFIG_NAME};
use starbase::system;
use starbase_styles::color;
use std::env;
use std::path::PathBuf;
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct RemoveToolArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(
        long,
        help = "Remove from the global user config instead of local .prototools"
    )]
    global: bool,
}

#[system]
pub async fn remove_tool(args: ArgsRef<RemoveToolArgs>) {
    if args.global {
        let mut user_config = UserConfig::load()?;
        user_config.plugins.remove(&args.id);
        user_config.save()?;

        info!(
            "Removed plugin {} from global {}",
            color::id(&args.id),
            color::path(&user_config.path),
        );

        return Ok(());
    }

    let local_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let config_path = local_path.join(TOOLS_CONFIG_NAME);

    if !config_path.exists() {
        return Err(ProtoCliError::MissingToolsConfigInCwd { path: config_path }.into());
    }

    let mut config = ToolsConfig::load_from(local_path)?;
    config.plugins.remove(&args.id);
    config.save()?;

    info!(
        "Removed plugin {} from local {}",
        color::id(&args.id),
        color::path(&config.path)
    );
}
