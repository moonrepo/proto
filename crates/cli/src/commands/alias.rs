use crate::error::ProtoCliError;
use clap::Args;
use proto_core::{is_alias_name, load_tool, Id, UnresolvedVersionSpec, UserConfig};
use starbase::system;
use starbase_styles::color;
use tracing::info;

#[derive(Args, Clone, Debug)]
pub struct AliasArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Alias name")]
    alias: String,

    #[arg(required = true, help = "Version or alias to associate with")]
    spec: UnresolvedVersionSpec,
}

#[system]
pub async fn alias(args: ArgsRef<AliasArgs>) {
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

    let tool = load_tool(&args.id).await?;
    let mut user_config = UserConfig::load()?;

    let tool_config = user_config.tools.entry(args.id.clone()).or_default();
    tool_config
        .aliases
        .insert(args.alias.clone(), args.spec.clone());

    user_config.save()?;

    info!(
        "Added alias {} ({}) for {}",
        color::id(&args.alias),
        color::muted_light(args.spec.to_string()),
        tool.get_name(),
    );
}
