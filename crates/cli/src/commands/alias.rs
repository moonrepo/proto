use crate::error::ProtoCliError;
use crate::session::{ProtoSession, SessionResult};
use clap::Args;
use proto_core::{
    PinLocation, ProtoConfig, ToolContext, ToolSpec, UnresolvedVersionSpec, cfg,
    version_spec::parse_alias,
};
use starbase_console::ui::*;
use starbase_styles::encode_style_tags;
use tracing::instrument;

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

#[instrument(skip(session))]
pub async fn alias(session: ProtoSession, args: AliasArgs) -> SessionResult {
    if let UnresolvedVersionSpec::Alias(inner_alias) = &args.spec.req
        && args.alias == inner_alias
    {
        return Err(ProtoCliError::AliasNoMatchingToVersion.into());
    }

    if parse_alias(&args.alias).is_err() {
        return Err(ProtoCliError::AliasInvalidName {
            alias: args.alias.clone(),
        }
        .into());
    }

    let tool = session.load_tool(&args.context).await?;

    let config_path = ProtoConfig::update_document(tool.proto.get_config_dir(args.to), |doc| {
        let tools = doc["tools"].or_insert(cfg::implicit_table());
        let record = tools[tool.context.as_str()].or_insert(cfg::implicit_table());
        let aliases = record["aliases"].or_insert(cfg::implicit_table());

        aliases[&args.alias] = cfg::value(args.spec.to_string());
    })?;

    session.console.notice(
        Variant::Success,
        format!(
            "Added <id>{}</id> alias <id>{}</id> <mutedlight>(with specification <versionalt>{}</versionalt>)</mutedlight> to config <path>{}</path>",
            args.context,
            args.alias,
            encode_style_tags(args.spec.to_string()),
            config_path.display()
        ),
    )?;

    Ok(None)
}
