use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{Id, ProtoConfig};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct UnpinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(long, help = "Location of .prototools to unpin from")]
    pub from: Option<PinOption>,
}

#[tracing::instrument(skip_all)]
pub async fn unpin(session: ProtoSession, args: UnpinArgs) -> AppResult {
    let tool = session.load_tool(&args.id).await?;
    let mut value = None;

    let config_path = ProtoConfig::update(
        tool.proto.get_config_dir(map_pin_type(false, args.from)),
        |config| {
            if let Some(versions) = &mut config.versions {
                value = versions.remove(&tool.id);
            }

            // Remove these also just in case
            if let Some(versions) = &mut config.unknown {
                versions.remove(tool.id.as_str());
            }
        },
    )?;

    let Some(value) = value else {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: format!(
                        "No version pinned for <id>{}</id> in config <path>{}</path>",
                        args.id,
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
                    "Removed <id>{}</id> version <hash>{}</hash> from config <path>{}</path>",
                    args.id,
                    value.to_string(),
                    config_path.display()
                ),
            )
        }
    })?;

    Ok(None)
}
