use clap::Args;
use proto_core::{detect_version, load_tool, Id, UnresolvedVersionSpec};
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(help = "Version or alias of tool")]
    spec: Option<UnresolvedVersionSpec>,

    #[arg(long, help = "Display shim path when available")]
    shim: bool,
}

#[system]
pub async fn bin(args: ArgsRef<BinArgs>) {
    let mut tool = load_tool(&args.id).await?;
    let version = detect_version(&tool, args.spec.clone()).await?;

    tool.resolve_version(&version).await?;
    tool.create_executables(true, false).await?;

    if args.shim {
        if let Some(shim_path) = tool.get_shim_path() {
            println!("{}", shim_path.to_string_lossy());

            return Ok(());
        }
    }

    println!("{}", tool.get_bin_path()?.to_string_lossy());
}
