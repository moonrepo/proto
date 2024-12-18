use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use iocraft::element;
use proto_core::{Id, ProtoConfig};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct UnaliasArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Alias name")]
    alias: String,

    #[arg(long, help = "Location of .prototools to remove from")]
    from: Option<PinOption>,
}

#[tracing::instrument(skip_all)]
pub async fn unalias(session: ProtoSession, args: UnaliasArgs) -> AppResult {
    let tool = session.load_tool(&args.id).await?;
    let mut value = None;

    let config_path = ProtoConfig::update(
        tool.proto.get_config_dir(map_pin_type(false, args.from)),
        |config| {
            if let Some(tool_configs) = &mut config.tools {
                if let Some(tool_config) = tool_configs.get_mut(&tool.id) {
                    if let Some(aliases) = &mut tool_config.aliases {
                        value = aliases.remove(&args.alias);
                    }
                }
            }
        },
    )?;

    let Some(value) = value else {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: format!(
                        "Alias <label>{}</label> for <id>{}</id> not found in config <path>{}</path>",
                        args.alias,
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
                    "Removed <id>{}</id> alias <label>{}</label> <mutedlight>({})</mutedlight> from config <path>{}</path>",
                    args.id,
                    args.alias,
                    value.to_string(),
                    config_path.display()
                ),
            )
        }
    })?;

    Ok(None)
}
