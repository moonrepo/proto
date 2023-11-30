use clap::Args;
use proto_core::{Id, PluginLocator, ProtoConfigManager, ProtoEnvironment};
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
    let proto = ProtoEnvironment::new()?;

    let config_path = ProtoConfigManager::update(
        if args.global { &proto.home } else { &proto.cwd },
        |config| {
            config
                .plugins
                .get_or_insert(Default::default())
                .insert(args.id.clone(), args.plugin.clone());
        },
    )?;

    info!(
        "Added plugin {} to config {}",
        color::id(&args.id),
        color::path(config_path)
    );
}
