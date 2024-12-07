use crate::error::ProtoCliError;
use crate::helpers::{map_pin_type, PinOption};
use crate::session::ProtoSession;
use clap::Args;
use iocraft::prelude::element;
use proto_core::{is_alias_name, Id, ProtoConfig, UnresolvedVersionSpec};
use starbase::AppResult;
use starbase_console::ui::*;

#[derive(Args, Clone, Debug)]
pub struct AliasArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Alias name")]
    alias: String,

    #[arg(required = true, help = "Version or alias to associate with")]
    spec: UnresolvedVersionSpec,

    #[arg(long, help = "Location of .prototools to add to")]
    to: Option<PinOption>,
}

#[tracing::instrument(skip_all)]
pub async fn alias(session: ProtoSession, args: AliasArgs) -> AppResult {
    if let UnresolvedVersionSpec::Alias(inner_alias) = &args.spec {
        if args.alias == inner_alias {
            return Err(ProtoCliError::NoMatchingAliasToVersion.into());
        }
    }

    if !is_alias_name(&args.alias) {
        return Err(ProtoCliError::InvalidAliasName {
            alias: args.alias.clone(),
        }
        .into());
    }

    let tool = session.load_tool(&args.id).await?;

    let config_path = ProtoConfig::update(
        tool.proto.get_config_dir(map_pin_type(false, args.to)),
        |config| {
            let tool_configs = config.tools.get_or_insert(Default::default());

            tool_configs
                .entry(tool.id.clone())
                .or_default()
                .aliases
                .get_or_insert(Default::default())
                .insert(args.alias.clone(), args.spec.clone());
        },
    )?;

    session.console.render(element! {
        Notice(variant: Variant::Success) {
            StyledText(
                content: format!(
                    "Added <id>{}</id> alias <id>{}</id> <mutedlight>({})</mutedlight> to config <path>{}</path>",
                    args.id,
                    args.alias,
                    args.spec.to_string(),
                    config_path.display()
                ),
            )
        }
    })?;

    Ok(None)
}
