use crate::session::ProtoSession;
use clap::Args;
use proto_core::{Id, ToolSpec};
use starbase::AppResult;

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(long, help = "Display symlinked binary path when available")]
    bin: bool,

    #[arg(help = "Version specification to locate")]
    spec: Option<ToolSpec>,

    #[arg(long, help = "Display shim path when available")]
    shim: bool,
}

#[tracing::instrument(skip_all)]
pub async fn bin(session: ProtoSession, args: BinArgs) -> AppResult {
    let mut tool = session
        .load_tool(&args.id, args.spec.clone().and_then(|spec| spec.backend))
        .await?;

    let spec = match args.spec.clone() {
        Some(spec) => spec,
        None => tool.detect_version().await?,
    };

    tool.resolve_version(&spec, true).await?;

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
