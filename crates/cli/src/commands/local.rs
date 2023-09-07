use clap::Args;
use proto_core::{load_tool, Id, ToolsConfig, UnresolvedVersionSpec};
use starbase::system;
use starbase_styles::color;
use std::{env, path::PathBuf};
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct LocalArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    spec: UnresolvedVersionSpec,
}

#[system]
pub async fn local(args: ArgsRef<LocalArgs>) {
    let tool = load_tool(&args.id).await?;
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
