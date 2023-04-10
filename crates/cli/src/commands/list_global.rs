use crate::tools::{create_tool, ToolType};
use human_sort::compare;
use proto_core::color;
use starbase::{diagnose::IntoDiagnostic, SystemResult};
use starbase_utils::fs;
use tracing::{debug, info};

pub async fn list_global(tool_type: ToolType) -> SystemResult {
    let tool = create_tool(&tool_type)?;
    let bin_dir = tool.get_globals_bin_dir()?;

    debug!("Finding globals from {}", color::path(&bin_dir));

    let mut bins = vec![];

    for file in fs::read_dir(&bin_dir)? {
        if file.file_type().into_diagnostic()?.is_dir() {
            continue;
        }

        let file_path = file.path();
        let mut file_name = fs::file_name(&file_path);

        match tool_type {
            ToolType::Rust => {
                if let Some(cargo_bin) = file_name.strip_prefix("cargo-") {
                    file_name = cargo_bin.to_owned();
                } else {
                    // Non-cargo binaries are in this directory
                    continue;
                }
            }
            _ => {
                // Do nothing!
            }
        }

        bins.push(format!(
            "{} - {}",
            file_name,
            color::path(file_path.canonicalize().unwrap())
        ));
    }

    info!("Installed globals:");

    if bins.is_empty() {
        eprintln!("None installed");
    } else {
        bins.sort_by(|a, d| compare(a, d));

        println!("{}", bins.join("\n"));
    }

    Ok(())
}
