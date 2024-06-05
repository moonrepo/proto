use crate::error::ProtoCliError;
use crate::session::ProtoSession;
use clap::Args;
use proto_core::{is_alias_name, Id, ProtoConfig, UnresolvedVersionSpec};
use starbase::AppResult;
use starbase_styles::color;

#[derive(Args, Clone, Debug)]
pub struct AliasArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Alias name")]
    alias: String,

    #[arg(required = true, help = "Version or alias to associate with")]
    spec: UnresolvedVersionSpec,

    #[arg(
        long,
        help = "Add to the global ~/.proto/.prototools instead of local ./.prototools"
    )]
    global: bool,
}

pub async fn alias(session: ProtoSession, args: AliasArgs) -> AppResult {
    if let UnresolvedVersionSpec::Alias(inner_alias) = &args.spec {
        if &args.alias == inner_alias {
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

    let config_path = ProtoConfig::update(tool.proto.get_config_dir(args.global), |config| {
        let tool_configs = config.tools.get_or_insert(Default::default());

        tool_configs
            .entry(tool.id.clone())
            .or_default()
            .aliases
            .get_or_insert(Default::default())
            .insert(args.alias.clone(), args.spec.clone());
    })?;

    println!(
        "Added alias {} ({}) to config {}",
        color::id(&args.alias),
        color::muted_light(args.spec.to_string()),
        color::path(config_path)
    );

    Ok(())
}
