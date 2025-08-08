use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{PinLocation, ProtoConfig, ToolContext};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct UnpinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub context: ToolContext,

    #[arg(long, default_value_t, help = "Location of .prototools to unpin from")]
    pub from: PinLocation,
}

#[tracing::instrument(skip_all)]
pub async fn unpin(session: ProtoSession, args: UnpinArgs) -> AppResult {
    let tool = session.load_tool(&args.context).await?;
    let mut value = None;

    let config_path = ProtoConfig::update_document(tool.proto.get_config_dir(args.from), |doc| {
        value = doc.as_table_mut().remove(tool.context.as_str());

        // if let Some(versions) = &mut config.versions {
        //     value = versions.remove(&tool.id);
        // }

        // // Remove these also just in case
        // if let Some(versions) = &mut config.unknown {
        //     versions.remove(tool.id.as_str());
        // }
    })?;

    let Some(value) = value else {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: format!(
                        "No version pinned for <id>{}</id> in config <path>{}</path>",
                        args.context,
                        config_path.display()
                    ),
                )
            }
        })?;

        return Ok(Some(1));
    };

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!(
                    "Removed <id>{}</id> version <version>{}</version> from config <path>{}</path>",
                    args.context,
                    value.to_string(),
                    config_path.display()
                ),
            )
        }
    })?;

    Ok(None)
}
