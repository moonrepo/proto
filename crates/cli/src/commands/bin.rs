use clap::Args;
use proto_core::{detect_version, load_tool, Id, VersionType};
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(help = "Version or alias of tool")]
    semver: Option<VersionType>,

    #[arg(long, help = "Display shim path when available")]
    shim: bool,
}

#[system]
pub async fn bin(args: ArgsRef<BinArgs>) {
    let mut tool = load_tool(&args.id).await?;
    let version = detect_version(&tool, args.semver.clone()).await?;

    tool.resolve_version(&version).await?;
    tool.locate_bins().await?;

    if args.shim {
        tool.setup_shims(true).await?;

        if let Some(shim_path) = tool.get_shim_path() {
            println!("{}", shim_path.to_string_lossy());

            return Ok(());
        }
    }

    println!("{}", tool.get_bin_path()?.to_string_lossy());
}
