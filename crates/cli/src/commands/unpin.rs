use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use proto_core::flow::lock::Locker;
use proto_core::{PinLocation, ProtoConfig, ToolContext, ToolSpec, reporter::NoticeOutput};
use proto_pdk_api::{PluginFunction, UnpinVersionInput, UnpinVersionOutput};
use starbase_console::ui::*;
use starbase_styles::encode_style_tags;
use tracing::instrument;

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

#[instrument(skip(session))]
pub async fn unpin(session: ProtoSession, args: UnpinArgs) -> SessionResult {
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
                config_path = tool.to_real_path(file).to_path_buf();
                value = output.version.map(|version| version.to_string());
            } else {
                let mut messages = vec![format!(
                    "Failed to unpin a version for <id>{}</id>.",
                    args.context,
                )];

                if let Some(error) = output.error {
                    messages.push(error);
                }

                session.console.notice_with(NoticeOutput {
                    variant: Variant::Failure,
                    messages,
                    ..Default::default()
                })?;

                return Ok(Some(1));
            }
        } else {
            session.console.notice(
                Variant::Caution,
                format!(
                    "{} does not support unpinning from a native file. Remove <shell>--tool-native</shell> and try again.",
                    tool.get_name()
                ),
            )?;

            return Ok(Some(1));
        }
    } else {
        let mut removed_spec = None;

        config_path = ProtoConfig::update_document(config_dir, |doc| {
            value = doc
                .as_table_mut()
                .remove(tool.context.as_str())
                .map(|item| {
                    removed_spec = item.as_str().and_then(|value| ToolSpec::parse(value).ok());

                    item.to_string()
                });
        })?;

        // Remove records for the unpinned spec from the lockfile owned by
        // the modified config (no-op if the config has not enabled a lockfile)
        if let Some(removed) = removed_spec {
            Locker::for_config(&tool, &config_path).remove_spec_from_lockfile(&removed.req)?;
        }
    }

    let Some(value) = value else {
        session.console.notice(
            Variant::Caution,
            format!(
                "No version pinned for <id>{}</id> in config <path>{}</path>",
                args.context,
                config_path.display()
            ),
        )?;

        return Ok(Some(1));
    };

    session.console.notice(
        Variant::Success,
        format!(
            "Removed <id>{}</id> version <version>{}</version> from config <path>{}</path>",
            args.context,
            encode_style_tags(value),
            config_path.display()
        ),
    )?;

    Ok(None)
}
