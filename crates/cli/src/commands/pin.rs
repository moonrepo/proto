use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, PinLocation, ProtoConfig, Tool, ToolSpec};
use starbase::AppResult;
use starbase_console::ui::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(required = true, help = "Version specification to pin")]
    pub spec: ToolSpec,

    #[arg(long, help = "Resolve the version before pinning")]
    pub resolve: bool,

    #[arg(long, default_value_t, help = "Location of .prototools to pin to")]
    pub to: PinLocation,
}

pub async fn internal_pin(
    tool: &mut Tool,
    spec: &ToolSpec,
    pin_to: PinLocation,
) -> miette::Result<PathBuf> {
    let config_path = ProtoConfig::update(tool.proto.get_config_dir(pin_to), |config| {
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
    let mut spec = args.spec.clone();

    if args.resolve {
        let res = tool.resolve_version_with_spec(&spec, false).await?;

        spec.req = res.to_unresolved_spec();
        spec.res = Some(res);
    }

    let config_path = internal_pin(&mut tool, &spec, args.to).await?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: if spec != args.spec {
                    format!(
                        "Pinned <id>{}</id> version <version>{}</version> (resolved from <versionalt>{}</versionalt>) to config <path>{}</path>",
                        args.id,
                        spec,
                        args.spec,
                        config_path.display()
                    )
                } else {
                    format!(
                        "Pinned <id>{}</id> version <version>{}</version> to config <path>{}</path>",
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
