use crate::session::ProtoSession;
use clap::Args;
use iocraft::element;
use proto_core::{PinLocation, ProtoConfig, ToolContext};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct UnaliasArgs {
    #[arg(required = true, help = "Tool to unalias")]
    context: ToolContext,

    #[arg(required = true, help = "Alias name")]
    alias: String,

    #[arg(long, default_value_t, help = "Location of .prototools to remove from")]
    from: PinLocation,
}

#[tracing::instrument(skip_all)]
pub async fn unalias(session: ProtoSession, args: UnaliasArgs) -> AppResult {
    let tool = session.load_tool(&args.context).await?;
    let mut value = None;

    let config_path = ProtoConfig::update_document(tool.proto.get_config_dir(args.from), |doc| {
        if let Some(tools) = doc.get_mut("tools").and_then(|item| item.as_table_mut()) {
            if let Some(record) = tools
                .get_mut(tool.get_id().as_str())
                .and_then(|item| item.as_table_mut())
            {
                if let Some(aliases) = record
                    .get_mut("aliases")
                    .and_then(|item| item.as_table_mut())
                {
                    value = aliases.remove(&args.alias);

                    if aliases.is_empty() {
                        record.remove("aliases");
                    }
                }

                if record.is_empty() {
                    tools.remove(tool.get_id().as_str());
                }
            }

            if tools.is_empty() {
                doc.as_table_mut().remove("tools");
            }
        }

        // if let Some(tool_configs) = &mut config.tools {
        //     if let Some(tool_config) = tool_configs.get_mut(&tool.id) {
        //         if let Some(aliases) = &mut tool_config.aliases {
        //             value = aliases.remove(&args.alias);
        //         }
        //     }
        // }
    })?;

    let Some(value) = value else {
        session.console.render(element! {
            Notice(variant: Variant::Caution) {
                StyledText(
                    content: format!(
                        "Alias <id>{}</id> for <id>{}</id> not found in config <path>{}</path>",
                        args.alias,
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
                    "Removed <id>{}</id> alias <id>{}</id> <mutedlight>(with specification <versionalt>{}</versionalt>)</mutedlight> from config <path>{}</path>",
                    args.context,
                    args.alias,
                    value.to_string(),
                    config_path.display()
                ),
            )
        }
    })?;

    Ok(None)
}
