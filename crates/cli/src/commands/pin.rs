use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, PinLocation, ProtoConfig, Tool, UnresolvedVersionSpec};
use starbase::AppResult;
use starbase_console::ui::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    pub spec: UnresolvedVersionSpec,

    #[arg(long, help = "Resolve the version before pinning")]
    pub resolve: bool,

    #[arg(long, help = "Location of .prototools to pin to")]
    pub to: Option<PinOption>,
}

pub async fn internal_pin(
    tool: &mut Tool,
    spec: &UnresolvedVersionSpec,
    pin: PinLocation,
) -> miette::Result<PathBuf> {
    let config_path = ProtoConfig::update(tool.proto.get_config_dir(pin), |config| {
        config
            .versions
            .get_or_insert(BTreeMap::default())
            .insert(tool.id.clone(), spec.clone());
    })?;

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
        tool.resolve_version(&args.spec, false)
            .await?
            .to_unresolved_spec()
    } else {
        args.spec.clone()
    };

    let config_path = internal_pin(&mut tool, &spec, map_pin_type(false, args.to)).await?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: if spec != args.spec {
                    format!(
                        "Pinned <id>{}</id> version <hash>{}</hash> (resolved from <hash>{}</hash>) to config <path>{}</path>",
                        args.id,
                        spec,
                        args.spec,
                        config_path.display()
                    )
                } else {
                    format!(
                        "Pinned <id>{}</id> version <hash>{}</hash> to config <path>{}</path>",
                        args.id,
                        args.spec,
                        config_path.display()
                    )
                },
            )
        }
    })?;

    Ok(None)
}
