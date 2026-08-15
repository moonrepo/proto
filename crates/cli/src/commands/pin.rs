use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use proto_core::flow::lock::{Locker, ProtoLockError};
use proto_core::flow::resolve::Resolver;
use proto_core::{
    PinLocation, ProtoConfig, Tool, ToolContext, ToolSpec, cfg, reporter::NoticeOutput,
};
use proto_pdk_api::{PinVersionInput, PinVersionOutput, PluginFunction};
use starbase_console::ui::*;
use starbase_styles::encode_style_tags;
use std::path::PathBuf;
use tracing::{debug, instrument};

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "Tool to pin")]
    pub context: ToolContext,

    #[arg(required = true, help = "Version specification to pin")]
    pub spec: ToolSpec,

    #[arg(long, help = "Resolve the version before pinning")]
    pub resolve: bool,

    #[arg(long, default_value_t, help = "Directory location to pin to")]
    pub to: PinLocation,

    #[arg(long, help = "Pin to the tool's native file instead of .prototools")]
    pub tool_native: bool,
}

pub async fn internal_pin(
    tool: &Tool,
    spec: &ToolSpec,
    pin_to: PinLocation,
) -> Result<PathBuf, ProtoLockError> {
    let version = match &spec.version {
        Some(version) => version.to_string(),
        None => spec.req.to_string(),
    };

    let config_dir = tool.proto.get_config_dir(pin_to);
    let mut previous_spec = None;

    let config_path = ProtoConfig::update_document(config_dir, |doc| {
        previous_spec = doc
            .get(tool.context.as_str())
            .and_then(|item| item.as_str())
            .and_then(|value| ToolSpec::parse(value).ok());

        doc[tool.context.as_str()] = cfg::value(&version);
    })?;

    debug!(
        tool = tool.context.as_str(),
        version = &version,
        config = ?config_path,
        "Pinned the version",
    );

    // Keep records in the lockfile owned by the modified config in sync
    // with the change (no-op if the config has not enabled a lockfile)
    let locker = Locker::for_config(tool, &config_path);

    match &spec.version {
        // We know what the pinned version resolves to, so migrate
        // records from the requested and previous specs to it
        Some(new_version) => {
            let new_spec = new_version.to_unresolved_spec();

            locker.update_spec_in_lockfile(&spec.req, &new_spec, new_version)?;

            if let Some(previous) = previous_spec
                && previous.req != spec.req
            {
                locker.update_spec_in_lockfile(&previous.req, &new_spec, new_version)?;
            }
        }
        // We don't know what the pinned version resolves to, so
        // remove records for the previous spec
        None => {
            if let Some(previous) = previous_spec
                && previous.req != spec.req
            {
                locker.remove_spec_from_lockfile(&previous.req)?;
            }
        }
    };

    Ok(config_path)
}

#[instrument(skip(session))]
pub async fn pin(session: ProtoSession, args: PinArgs) -> SessionResult {
    let mut spec = args.spec.clone();
    let tool = session.load_tool(&args.context).await?;

    if args.resolve {
        Resolver::resolve(&tool, &mut spec, false).await?;
    }

    let config_path;

    if args.tool_native {
        if tool.plugin.has_func(PluginFunction::PinVersion).await {
            let output: PinVersionOutput = tool
                .plugin
                .call_func_with(
                    PluginFunction::PinVersion,
                    PinVersionInput {
                        context: tool.create_plugin_unresolved_context(),
                        dir: tool.to_virtual_path(tool.proto.get_config_dir(args.to)),
                        version: spec.to_unresolved_spec(),
                    },
                )
                .await?;

            if let Some(file) = output.file
                && output.pinned
            {
                config_path = tool.to_real_path(file).to_path_buf();
            } else {
                let mut messages = vec![format!(
                    "Failed to pin version <version>{}</version> for <id>{}</id>.",
                    encode_style_tags(spec.to_string()),
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
                    "{} does not support pinning to a native file. Remove <shell>--tool-native</shell> and try again.",
                    tool.get_name()
                ),
            )?;

            return Ok(Some(1));
        }
    } else {
        config_path = internal_pin(&tool, &spec, args.to).await?;
    }

    session.console.notice(
        Variant::Success,
        if args.resolve {
            format!(
                "Pinned <id>{}</id> version <version>{}</version> (resolved from <versionalt>{}</versionalt>) to config <path>{}</path>",
                args.context,
                spec.get_resolved_version(),
                encode_style_tags(spec.req.to_string()),
                config_path.display()
            )
        } else {
            format!(
                "Pinned <id>{}</id> version <version>{}</version> to config <path>{}</path>",
                args.context,
                encode_style_tags(spec.req.to_string()),
                config_path.display()
            )
        },
    )?;

    Ok(None)
}
