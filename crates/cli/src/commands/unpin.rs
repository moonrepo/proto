use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{PinLocation, ProtoConfig, ToolContext};
use proto_pdk_api::{PluginFunction, UnpinVersionInput, UnpinVersionOutput};
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_styles::encode_style_tags;

#[derive(Args, Clone, Debug)]
pub struct UnpinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub context: ToolContext,

    #[arg(long, default_value_t, help = "Directory location to unpin from")]
    pub from: PinLocation,

    #[arg(
        long,
        help = "Unpin from the tool's native file instead of .prototools"
    )]
    pub tool_native: bool,
}

#[tracing::instrument(skip_all)]
pub async fn unpin(session: ProtoSession, args: UnpinArgs) -> AppResult {
    let tool = session.load_tool(&args.context).await?;
    let mut value = None;
    let config_dir = tool.proto.get_config_dir(args.from);
    let config_path;

    if args.tool_native {
        if tool.plugin.has_func(PluginFunction::UnpinVersion).await {
            let output: UnpinVersionOutput = tool
                .plugin
                .call_func_with(
                    PluginFunction::UnpinVersion,
                    UnpinVersionInput {
                        context: tool.create_plugin_unresolved_context(),
                        dir: tool.to_virtual_path(config_dir),
                    },
                )
                .await?;

            if let Some(file) = output.file
                && output.unpinned
            {
                config_path = tool.from_virtual_path(file);
                value = output.version.map(|version| version.to_string());
            } else {
                session.console.render_err(element! {
                    Notice(variant: Variant::Failure) {
                        StyledText(
                            content: format!(
                                "Failed to unpin a version for <id>{}</id>.",
                                args.context,
                            )
                        )
                        #(output.error.map(|error| {
                            element! {
                                StyledText(content: error)
                            }
                        }))
                    }
                })?;

                return Ok(Some(1));
            }
        } else {
            session.console.render_err(element! {
                Notice(variant: Variant::Caution) {
                    StyledText(
                        content: format!(
                            "{} does not support unpinning from a native file. Remove <shell>--tool-native</shell> and try again.",
                            tool.get_name()
                        )
                    )
                }
            })?;

            return Ok(Some(1));
        }
    } else {
        config_path = ProtoConfig::update_document(config_dir, |doc| {
            value = doc
                .as_table_mut()
                .remove(tool.context.as_str())
                .map(|item| item.to_string());
        })?;
    }

    let Some(value) = value else {
        session.console.render_err(element! {
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
                    encode_style_tags(value),
                    config_path.display()
                ),
            )
        }
    })?;

    Ok(None)
}
