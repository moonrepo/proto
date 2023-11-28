use clap::Args;
use proto_core::{
    load_tool, Id, Tool, ToolsConfig, UnresolvedVersionSpec, UserConfig, UserToolConfig,
};
use starbase::{system, SystemResult};
use starbase_styles::color;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "ID of tool")]
    pub id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    pub spec: UnresolvedVersionSpec,

    #[arg(
        long,
        help = "Add to the global user config instead of local .prototools"
    )]
    pub global: bool,
}

pub async fn internal_pin(tool: &mut Tool, args: &PinArgs, link: bool) -> SystemResult {
    if args.global {
        let mut user_config = UserConfig::load()?;

        user_config
            .tools
            .entry(tool.id.clone())
            .and_modify(|cfg| {
                cfg.default_version = Some(args.spec.clone());
            })
            .or_insert_with(|| UserToolConfig {
                default_version: Some(args.spec.clone()),
                ..UserToolConfig::default()
            });

        user_config.save()?;

        debug!(
            version = args.spec.to_string(),
            config = ?user_config.path,
            "Wrote the global version",
        );

        // Create symlink to this new version
        if link {
            tool.symlink_bins(true).await?;
        }
    } else {
        let mut config = ToolsConfig::load()?;
        config.tools.insert(args.id.clone(), args.spec.clone());
        config.save()?;

        debug!(
            version = args.spec.to_string(),
            config = ?config.path,
            "Wrote the local version",
        );
    }

    Ok(())
}

#[system]
pub async fn pin(args: ArgsRef<PinArgs>) -> SystemResult {
    let mut tool = load_tool(&args.id).await?;

    internal_pin(&mut tool, args, false).await?;

    info!(
        "Set the {} version to {}",
        tool.get_name(),
        color::hash(args.spec.to_string())
    );
}
