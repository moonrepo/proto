use clap::Args;
use proto_core::{Id, ToolsConfig, UserConfig, TOOLS_CONFIG_NAME};
use starbase::system;
use starbase_styles::color;
use std::path::PathBuf;
use std::{env, process};
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct RemovePluginArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

    #[arg(
        long,
        help = "Remove from the global user config instead of local .prototools"
    )]
    global: bool,
}

#[system]
pub async fn remove_plugin(args: ArgsRef<RemovePluginArgs>) {
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
        eprintln!(
            "No {} found in the current directory",
            color::file(TOOLS_CONFIG_NAME)
        );

        process::exit(1);
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
