use crate::error::ProtoCliError;
use crate::helpers::ProtoResource;
use clap::Args;
use proto_core::{detect_version, Id};
use starbase::diagnostics::IntoDiagnostic;
use starbase::system;
use starbase_styles::color;
use starbase_utils::fs;
use std::process;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct ListGlobalArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,
}

#[system]
pub async fn list_global(args: ArgsRef<ListGlobalArgs>, proto: ResourceRef<ProtoResource>) {
    let mut tool = proto.load_tool(&args.id).await?;
    let version = detect_version(&tool, None).await?;

    // Resolve a version as some tools install to a versioned folder
    tool.resolve_version(&version, true).await?;
    tool.locate_globals_dir().await?;

    let Some(globals_dir) = tool.get_globals_bin_dir() else {
        return Err(ProtoCliError::GlobalsNotSupported {
            tool: tool.get_name().to_owned(),
        }
        .into());
    };

    debug!(globals_dir = ?globals_dir, "Finding global packages");

    let mut bins = vec![];
    let globals_prefix = tool.get_globals_prefix();

    if globals_dir.exists() {
        for file in fs::read_dir(globals_dir)? {
            if file.file_type().into_diagnostic()?.is_dir() {
                continue;
            }

            let file_path = file.path();
            let mut file_name = fs::file_name(&file_path);

            if let Some(prefix) = globals_prefix {
                if let Some(prefixless) = file_name.strip_prefix(prefix) {
                    file_name = prefixless.to_owned();
                } else {
                    continue;
                }
            }

            bins.push(format!(
                "{} - {}",
                file_name,
                color::path(file_path.canonicalize().unwrap_or(file_path))
            ));
        }
    }

    if bins.is_empty() {
        eprintln!("No global packages installed");
        process::exit(1);
    }

    bins.sort();

    println!("{}", bins.join("\n"));
}
