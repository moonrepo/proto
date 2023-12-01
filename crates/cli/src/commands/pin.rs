use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::{Id, ProtoConfig, Tool, UnresolvedVersionSpec};
use starbase::{system, SystemResult};
use starbase_styles::color;
use std::collections::BTreeMap;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    pub spec: UnresolvedVersionSpec,

    #[arg(
        long,
        help = "Pin to the global .prototools instead of local .prototools"
    )]
    pub global: bool,
}

pub async fn internal_pin(tool: &mut Tool, args: &PinArgs, link: bool) -> SystemResult {
    // Create symlink to this new version
    if args.global && link {
        tool.symlink_bins(true).await?;
    }

    let path = ProtoConfig::update(tool.proto.get_config_dir(args.global), |config| {
        config
            .versions
            .get_or_insert(BTreeMap::default())
            .insert(args.id.clone(), args.spec.clone());
    })?;

    debug!(
        version = args.spec.to_string(),
        config = ?path,
        "Pinned the version",
    );

    Ok(())
}

#[system]
pub async fn pin(args: ArgsRef<PinArgs>, proto: ResourceRef<ProtoResource>) -> SystemResult {
    let mut tool = proto.load_tool(&args.id).await?;

    internal_pin(&mut tool, args, false).await?;

    info!(
        "Set the {} version to {}",
        tool.get_name(),
        color::hash(args.spec.to_string())
    );
}
