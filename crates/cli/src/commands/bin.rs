use crate::session::ProtoSession;
use clap::{Args, ValueEnum};
use proto_core::flow::resolve::Resolver;
use proto_core::{ToolContext, ToolSpec};
use starbase::AppResult;

#[derive(Clone, Debug, ValueEnum)]
enum BinDirType {
    Exes,
    Globals,
}

#[derive(Args, Clone, Debug)]
pub struct BinArgs {
    #[arg(required = true, help = "Tool to inspect")]
    context: ToolContext,

    #[arg(long, help = "List all paths instead of just one")]
    all: bool,

    #[arg(
        value_enum,
        long,
        help = "Display the chosen directory path if available"
    )]
    dir: Option<BinDirType>,

    #[arg(long, help = "Display symlinked binary path when available")]
    bin: bool,

    #[arg(help = "Version specification to locate")]
    spec: Option<ToolSpec>,

    #[arg(long, help = "Display shim path when available")]
    shim: bool,
}

#[tracing::instrument(skip_all)]
pub async fn bin(session: ProtoSession, args: BinArgs) -> AppResult {
    let mut tool = session.load_tool(&args.context).await?;

    let mut spec = match args.spec.clone() {
        Some(spec) => spec,
        None => tool.detect_version().await?,
    };

    Resolver::new(&tool)
        .resolve_version(&mut spec, true)
        .await?;

    if args.bin {
        tool.symlink_bins(true).await?;

        for bin in tool.resolve_bin_locations(None).await? {
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
        tool.generate_shims(&spec, true).await?;

        for shim in tool.resolve_shim_locations(&spec).await? {
            if shim.config.primary {
                session
                    .console
                    .out
                    .write_line(shim.path.display().to_string())?;

                return Ok(None);
            }
        }
    }

    let paths = match args.dir {
        None => vec![tool.locate_exe_file(&spec).await?],
        Some(BinDirType::Exes) => tool.locate_exes_dirs(&spec).await?,
        Some(BinDirType::Globals) => tool.locate_globals_dirs(&spec).await?,
    };

    if args.all {
        for path in paths {
            session.console.out.write_line(path.display().to_string())?;
        }
    } else if let Some(path) = paths.first() {
        session.console.out.write_line(path.display().to_string())?;
    }

    Ok(None)
}
