use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::{Id, PluginLocator, ProtoConfig};
use starbase::system;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct AddPluginArgs {
    #[arg(required = true, help = "ID of plugin")]
    id: Id,

    #[arg(required = true, help = "Locator string to find and load the plugin")]
    plugin: PluginLocator,

    #[arg(
        long,
        help = "Add to the global ~/.proto/.prototools instead of local ./.prototools"
    )]
    global: bool,
}

#[system]
pub async fn add(args: ArgsRef<AddPluginArgs>, proto: ResourceRef<ProtoResource>) {
    let config_path = ProtoConfig::update(proto.env.get_config_dir(args.global), |config| {
        config
            .plugins
            .get_or_insert(Default::default())
            .insert(args.id.clone(), args.plugin.clone());
    })?;

    println!(
        "Added plugin {} to config {}",
        color::id(&args.id),
        color::path(config_path)
    );
}
