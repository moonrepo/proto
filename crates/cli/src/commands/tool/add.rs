use clap::Args;
use proto_core::{Id, PluginLocator, ToolsConfig, UserConfig};
use starbase::system;
use starbase_styles::color;
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct AddToolArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Locator string to find and load the plugin")]
    plugin: PluginLocator,

    #[arg(
        long,
        help = "Add to the global user config instead of local .prototools"
    )]
    global: bool,
}

#[system]
pub async fn add(args: ArgsRef<AddToolArgs>) {
    if args.global {
        let mut user_config = UserConfig::load()?;
        user_config
            .plugins
            .insert(args.id.clone(), args.plugin.clone());
        user_config.save()?;

        info!(
            "Added plugin {} to global {}",
            color::id(&args.id),
            color::path(&user_config.path),
        );

        return Ok(());
    }

    let mut config = ToolsConfig::load()?;
    config.plugins.insert(args.id.clone(), args.plugin.clone());
    config.save()?;

    info!(
        "Added plugin {} to local {}",
        color::id(&args.id),
        color::path(&config.path)
    );
}
