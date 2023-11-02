use clap::Args;
use proto_core::{detect_version, load_tool, Id, UnresolvedVersionSpec};
use starbase::system;

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(long, help = "Display symlinked binary path when available")]
    bin: bool,

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

    if args.bin {
        for location in tool.get_bin_locations()? {
            if location.primary {
                println!("{}", location.path.display());
                return Ok(());
            }
        }
    }

    if args.shim {
        for location in tool.get_shim_locations()? {
            if location.primary {
                println!("{}", location.path.display());
                return Ok(());
            }
        }
    }

    println!("{}", tool.get_exe_path()?.display());
}
