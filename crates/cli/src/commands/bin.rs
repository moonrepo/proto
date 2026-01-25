use crate::session::{LoadToolOptions, ProtoSession};
use clap::{Args, ValueEnum};
use proto_core::flow::locate::Locator;
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
    let mut tool = session
        .load_tool_with_options(
            &args.context,
            LoadToolOptions {
                detect_version: args.spec.is_none(),
                ..Default::default()
            },
        )
        .await?;

    let mut spec = args
        .spec
        .clone()
        .or_else(|| tool.detected_version.clone())
        .unwrap_or_else(|| tool.spec.clone());

    Resolver::new(&tool)
        .resolve_version(&mut spec, true)
        .await?;

    if args.bin {
        tool.symlink_bins(&spec, true).await?;

        for bin in Locator::new(&tool, &spec).locate_bins(None).await? {
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

        for shim in Locator::new(&tool, &spec).locate_shims().await? {
            if shim.config.primary {
                session
                    .console
                    .out
                    .write_line(shim.path.display().to_string())?;

                return Ok(None);
            }
        }
    }

    let mut locator = Locator::new(&tool, &spec);

    let paths = match args.dir {
        None => vec![locator.locate_exe_file().await?],
        Some(BinDirType::Exes) => locator.locate_exes_dirs().await?,
        Some(BinDirType::Globals) => locator.locate_globals_dirs().await?,
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
