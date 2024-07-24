use crate::session::ProtoSession;
use clap::Args;
use proto_core::{detect_version, Id, UnresolvedVersionSpec};
use proto_shim::{get_exe_file_name, locate_proto_exe};
use starbase::AppResult;

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

#[tracing::instrument(skip_all)]
pub async fn bin(session: ProtoSession, args: BinArgs) -> AppResult {
    if args.id == "proto" {
        println!(
            "{}",
            locate_proto_exe("proto")
                .unwrap_or(session.env.store.bin_dir.join(get_exe_file_name("proto")))
                .display()
        );

        return Ok(());
    }

    let mut tool = session.load_tool(&args.id).await?;
    let version = detect_version(&tool, args.spec.clone()).await?;

    tool.resolve_version(&version, true).await?;
    tool.create_executables(args.shim, args.bin).await?;

    if args.bin {
        for bin in tool.get_bin_locations()? {
            if bin.primary {
                println!("{}", bin.path.display());
                return Ok(());
            }
        }
    }

    if args.shim {
        for shim in tool.get_shim_locations()? {
            if shim.primary {
                println!("{}", shim.path.display());
                return Ok(());
            }
        }
    }

    println!("{}", tool.get_exe_path()?.display());

    Ok(())
}
