use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::{is_alias_name, Id, ProtoConfig, UnresolvedVersionSpec};
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

    #[arg(
        long,
        help = "Add to the global .prototools instead of local .prototools"
    )]
    global: bool,
}

#[system]
pub async fn alias(args: ArgsRef<AliasArgs>, proto: ResourceRef<ProtoResource>) {
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

    let tool = proto.load_tool(&args.id).await?;

    ProtoConfig::update(tool.proto.get_config_dir(args.global), |config| {
        let tool_configs = config.tools.get_or_insert(Default::default());
        let tool_config = tool_configs.entry(tool.id.clone()).or_default();

        tool_config
            .aliases
            .get_or_insert(Default::default())
            .insert(args.alias.clone(), args.spec.clone());
    })?;

    info!(
        "Added alias {} ({}) for {}",
        color::id(&args.alias),
        color::muted_light(args.spec.to_string()),
        tool.get_name(),
    );
}
