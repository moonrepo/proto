use clap::Args;
use proto_core::{load_tool, Id, VersionType};
use starbase::system;
use starbase_styles::color;
use tracing::{debug, info};

#[derive(Args, Clone, Debug)]
pub struct GlobalArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(required = true, help = "Version or alias of tool")]
    semver: VersionType,
}

#[system]
pub async fn global(args: ArgsRef<GlobalArgs>) -> SystemResult {
    let mut tool = load_tool(&args.id).await?;

    tool.manifest.default_version = Some(args.semver.clone());
    tool.manifest.save()?;

    debug!(
        version = args.semver.to_string(),
        manifest = ?tool.manifest.path,
        "Wrote the global version",
    );

    info!(
        "Set the global {} version to {}",
        tool.get_name(),
        color::hash(args.semver.to_string())
    );
}
