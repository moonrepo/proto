use std::env;
use std::path::PathBuf;

use clap::Args;
use proto_core::{load_tool, Id, ToolsConfig, UnresolvedVersionSpec};
use starbase::system;
use starbase_styles::color;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct PinArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    spec: UnresolvedVersionSpec,

    #[arg(
        long,
        help = "Add to the global user config instead of local .prototools"
    )]
    global: bool,
}

#[system]
pub async fn pin(args: ArgsRef<PinArgs>) -> SystemResult {
    let mut tool = load_tool(&args.id).await?;

    if args.global {
        tool.manifest.default_version = Some(args.spec.clone());
        tool.manifest.save()?;

        debug!(
            version = args.spec.to_string(),
            manifest = ?tool.manifest.path,
            "Wrote the global version",
        );

        info!(
            "Set the global {} version to {}",
            tool.get_name(),
            color::hash(args.spec.to_string())
        );
    } else {
        let local_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        let mut config = ToolsConfig::load_from(local_path)?;
        config.tools.insert(args.id.clone(), args.spec.clone());
        config.save()?;

        debug!(
            version = args.spec.to_string(),
            config = ?config.path,
            "Wrote the local version",
        );

        info!(
            "Set the local {} version to {}",
            tool.get_name(),
            color::hash(args.spec.to_string())
        );
    }
}
