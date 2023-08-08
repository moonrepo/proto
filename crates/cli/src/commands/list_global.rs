use crate::tools::create_tool;
use starbase::{diagnostics::IntoDiagnostic, SystemResult};
use starbase_styles::color;
use starbase_utils::fs;
use std::process;
use tracing::debug;

pub async fn list_global(tool_id: String) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;
    tool.locate_globals_dir().await?;

    let Some(globals_dir) = tool.get_globals_bin_dir() else {
        eprintln!("{} does not support global packages", tool.get_name());
        process::exit(1);
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
                color::path(file_path.canonicalize().unwrap())
            ));
        }
    }

    if bins.is_empty() {
        eprintln!("No global packages installed");
        process::exit(1);
    }

    bins.sort();

    println!("{}", bins.join("\n"));

    Ok(())
}
