use crate::session::ProtoSession;
use clap::Args;
use proto_core::{detect_version, Id, UnresolvedVersionSpec, PROTO_PLUGIN_KEY};
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
    if args.id == PROTO_PLUGIN_KEY {
        session.console.out.write_line(
            locate_proto_exe("proto")
                .unwrap_or(session.env.store.bin_dir.join(get_exe_file_name("proto")))
                .display()
                .to_string(),
        )?;

        return Ok(None);
    }

    let mut tool = session.load_tool(&args.id).await?;
    let version = detect_version(&tool, args.spec.clone()).await?;

    tool.resolve_version(&version, true).await?;

    if args.bin {
        tool.symlink_bins(true).await?;

        for bin in tool.resolve_bin_locations(false).await? {
            if bin.config.primary {
                session
                    .console
                    .out
                    .write_line(bin.path.display().to_string())?;

                return Ok(None);
            }
        }
    }

    if args.shim {
        tool.generate_shims(true).await?;

        for shim in tool.resolve_shim_locations().await? {
            if shim.config.primary {
                session
                    .console
                    .out
                    .write_line(shim.path.display().to_string())?;

                return Ok(None);
            }
        }
    }

    session
        .console
        .out
        .write_line(tool.locate_exe_file().await?.display().to_string())?;

    Ok(None)
}
