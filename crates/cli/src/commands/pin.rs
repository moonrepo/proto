use crate::session::ProtoSession;
use clap::Args;
use proto_core::{Id, ProtoConfig, Tool, UnresolvedVersionSpec};
use starbase::AppResult;
use starbase_styles::color;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    pub spec: UnresolvedVersionSpec,

    #[arg(long, group = "pin", help = "Pin to the global ~/.proto/.prototools")]
    pub global: bool,

    #[arg(long, group = "pin", help = "Pin to the user ~/.prototools")]
    pub user: bool,

    #[arg(long, help = "Resolve the version before pinning")]
    pub resolve: bool,
}

pub async fn internal_pin(
    tool: &mut Tool,
    spec: &UnresolvedVersionSpec,
    global: bool,
    user: bool,
    link: bool,
) -> miette::Result<PathBuf> {
    // Create symlink to this new version
    if global && link {
        tool.symlink_bins(true).await?;
    }

    let config_path = ProtoConfig::update(
        tool.proto.get_config_dir_from_flags(global, user),
        |config| {
            config
                .versions
                .get_or_insert(BTreeMap::default())
                .insert(tool.id.clone(), spec.clone());
        },
    )?;

    debug!(
        version = spec.to_string(),
        config = ?config_path,
        "Pinned the version",
    );

    Ok(config_path)
}

#[tracing::instrument(skip_all)]
pub async fn pin(session: ProtoSession, args: PinArgs) -> AppResult {
    let mut tool = session.load_tool(&args.id).await?;

    let spec = if args.resolve {
        tool.resolve_version(&args.spec, false).await?;
        tool.get_resolved_version().to_unresolved_spec()
    } else {
        args.spec.clone()
    };

    let config_path = internal_pin(&mut tool, &spec, args.global, args.user, false).await?;

    println!(
        "Pinned {} to {} in {}",
        tool.get_name(),
        color::hash(args.spec.to_string()),
        color::path(config_path),
    );

    Ok(())
}
