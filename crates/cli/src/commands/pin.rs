use std::collections::BTreeMap;

use clap::Args;
use proto_core::{load_tool, Id, ProtoConfigManager, Tool, UnresolvedVersionSpec};
use starbase::{system, SystemResult};
use starbase_styles::color;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    pub spec: UnresolvedVersionSpec,

    #[arg(
        long,
        help = "Add to the global user config instead of local .prototools"
    )]
    pub global: bool,
}

pub async fn internal_pin(tool: &mut Tool, args: &PinArgs, link: bool) -> SystemResult {
    let dir = if args.global {
        // Create symlink to this new version
        if link {
            tool.symlink_bins(true).await?;
        }

        &tool.proto.home
    } else {
        &tool.proto.cwd
    };

    let mut config = ProtoConfigManager::load_from(dir)?;

    config
        .versions
        .get_or_insert(BTreeMap::default())
        .insert(args.id.clone(), args.spec.clone());

    let path = ProtoConfigManager::save_to(dir, config)?;

    debug!(
        version = args.spec.to_string(),
        config = ?path,
        "Pinned the version",
    );

    Ok(())
}

#[system]
pub async fn pin(args: ArgsRef<PinArgs>) -> SystemResult {
    let mut tool = load_tool(&args.id).await?;

    internal_pin(&mut tool, args, false).await?;

    info!(
        "Set the {} version to {}",
        tool.get_name(),
        color::hash(args.spec.to_string())
    );
}
