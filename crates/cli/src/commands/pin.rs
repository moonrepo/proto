use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{PinLocation, ProtoConfig, ProtoConfigError, Tool, ToolContext, ToolSpec, cfg};
use starbase::AppResult;
use starbase_console::ui::*;
use std::path::PathBuf;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "Tool to pin")]
    pub context: ToolContext,

    #[arg(required = true, help = "Version specification to pin")]
    pub spec: ToolSpec,

    #[arg(long, help = "Resolve the version before pinning")]
    pub resolve: bool,

    #[arg(long, default_value_t, help = "Location of .prototools to pin to")]
    pub to: PinLocation,
}

pub async fn internal_pin(
    tool: &Tool,
    spec: &ToolSpec,
    pin_to: PinLocation,
) -> Result<PathBuf, ProtoConfigError> {
    let config_path = ProtoConfig::update_document(tool.proto.get_config_dir(pin_to), |doc| {
        doc[tool.context.as_str()] = cfg::value(spec.to_string());

        // config
        //     .versions
        //     .get_or_insert(Default::default())
        //     .insert(tool.id.clone(), spec.clone());
    })?;

    debug!(
        tool = tool.context.as_str(),
        version = spec.to_string(),
        config = ?config_path,
        "Pinned the version",
    );

    Ok(config_path)
}

#[tracing::instrument(skip_all)]
pub async fn pin(session: ProtoSession, args: PinArgs) -> AppResult {
    let mut spec = args.spec.clone();
    let mut tool = session.load_tool(&args.context).await?;

    if args.resolve {
        spec.req = tool
            .resolve_version(&spec, false)
            .await?
            .to_unresolved_spec();
    }

    let config_path = internal_pin(&tool, &spec, args.to).await?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: if spec != args.spec {
                    format!(
                        "Pinned <id>{}</id> version <version>{}</version> (resolved from <versionalt>{}</versionalt>) to config <path>{}</path>",
                        args.context,
                        spec,
                        args.spec,
                        config_path.display()
                    )
                } else {
                    format!(
                        "Pinned <id>{}</id> version <version>{}</version> to config <path>{}</path>",
                        args.context,
                        args.spec,
                        config_path.display()
                    )
                },
            )
        }
    })?;

    Ok(None)
}
