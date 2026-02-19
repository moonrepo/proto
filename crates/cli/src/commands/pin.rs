use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::flow::resolve::Resolver;
use proto_core::{PinLocation, ProtoConfig, ProtoConfigError, Tool, ToolContext, ToolSpec, cfg};
use proto_pdk_api::{PinVersionInput, PinVersionOutput, PluginFunction};
use starbase::AppResult;
use starbase_console::ui::*;
use starbase_styles::encode_style_tags;
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

    #[arg(long, default_value_t, help = "Directory location to pin to")]
    pub to: PinLocation,

    #[arg(long, help = "Pin to the tool's native file instead of .prototools")]
    pub tool_native: bool,
}

pub async fn internal_pin(
    tool: &Tool,
    spec: &ToolSpec,
    pin_to: PinLocation,
) -> Result<PathBuf, ProtoConfigError> {
    let version = match &spec.version {
        Some(version) => version.to_string(),
        None => spec.req.to_string(),
    };

    let config_path = ProtoConfig::update_document(tool.proto.get_config_dir(pin_to), |doc| {
        doc[tool.context.as_str()] = cfg::value(&version);
    })?;

    debug!(
        tool = tool.context.as_str(),
        version = &version,
        config = ?config_path,
        "Pinned the version",
    );

    Ok(config_path)
}

#[tracing::instrument(skip_all)]
pub async fn pin(session: ProtoSession, args: PinArgs) -> AppResult {
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
                config_path = tool.from_virtual_path(file);
            } else {
                session.console.render_err(element! {
                    Notice(variant: Variant::Failure) {
                        StyledText(
                            content: format!(
                                "Failed to pin version <version>{}</version> for <id>{}</id>.",
                                encode_style_tags(spec.to_string()),
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
                            "{} does not support pinning to a native file. Remove <shell>--tool-native</shell> and try again.",
                            tool.get_name()
                        )
                    )
                }
            })?;

            return Ok(Some(1));
        }
    } else {
        config_path = internal_pin(&tool, &spec, args.to).await?;
    }

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: if args.resolve {
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
            )
        }
    })?;

    Ok(None)
}
