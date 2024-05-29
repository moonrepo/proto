use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::{Id, ProtoConfig};
use starbase::system;
use starbase_styles::color;
use std::process;

#[derive(Args, Clone, Debug)]
pub struct UnpinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(
        long,
        help = "Unpin from the global ~/.proto/.prototools instead of local ./.prototools"
    )]
    pub global: bool,
}

#[system]
pub async fn unpin(args: ArgsRef<UnpinArgs>, proto: ResourceRef<ProtoResource>) -> SystemResult {
    let tool = proto.load_tool(&args.id).await?;
    let mut value = None;

    let config_path = ProtoConfig::update(tool.proto.get_config_dir(args.global), |config| {
        if let Some(versions) = &mut config.versions {
            value = versions.remove(&tool.id);
        }

        // Remove also just in case
        if let Some(versions) = &mut config.unknown {
            versions.remove(tool.id.as_str());
        }
    })?;

    let Some(value) = value else {
        eprintln!("No version pinned in config {}", color::path(config_path));

        process::exit(1);
    };

    println!(
        "Removed version {} from config {}",
        color::hash(value.to_string()),
        color::path(config_path)
    );
}
