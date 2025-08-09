use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{
    PinLocation, ProtoConfig, ToolContext, ToolSpec, UnresolvedVersionSpec, cfg, is_alias_name,
};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct AliasArgs {
    #[arg(required = true, help = "Tool to alias")]
    context: ToolContext,

    #[arg(required = true, help = "Alias name")]
    alias: String,

    #[arg(required = true, help = "Version specification to alias")]
    spec: ToolSpec,

    #[arg(long, default_value_t, help = "Location of .prototools to add to")]
    to: PinLocation,
}

#[tracing::instrument(skip_all)]
pub async fn alias(session: ProtoSession, args: AliasArgs) -> AppResult {
    if let UnresolvedVersionSpec::Alias(inner_alias) = &args.spec.req
        && args.alias == inner_alias
    {
        return Err(ProtoCliError::AliasNoMatchingToVersion.into());
    }

    if !is_alias_name(&args.alias) {
        return Err(ProtoCliError::AliasInvalidName {
            alias: args.alias.clone(),
        }
        .into());
    }

    let tool = session.load_tool(&args.context).await?;

    let config_path = ProtoConfig::update_document(tool.proto.get_config_dir(args.to), |doc| {
        let tools = doc["tools"].or_insert(cfg::implicit_table());
        let record = tools[tool.get_id().as_str()].or_insert(cfg::implicit_table());
        let aliases = record["aliases"].or_insert(cfg::implicit_table());

        aliases[&args.alias] = cfg::value(args.spec.to_string());

        // let tool_configs = config.tools.get_or_insert(Default::default());

        // tool_configs
        //     .entry(tool.id.clone())
        //     .or_default()
        //     .aliases
        //     .get_or_insert(Default::default())
        //     .insert(args.alias.clone(), args.spec.clone());
    })?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!(
                    "Added <id>{}</id> alias <id>{}</id> <mutedlight>(with specification <versionalt>{}</versionalt>)</mutedlight> to config <path>{}</path>",
                    args.context,
                    args.alias,
                    args.spec.to_string(),
                    config_path.display()
                ),
            )
        }
    })?;

    Ok(None)
}
