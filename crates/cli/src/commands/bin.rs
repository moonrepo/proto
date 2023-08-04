use crate::tools::create_tool;
use proto_core::{detect_version, AliasOrVersion};
use starbase::SystemResult;

pub async fn bin(
    tool_id: String,
    forced_version: Option<AliasOrVersion>,
    use_shim: bool,
) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;
    let version = detect_version(&tool, forced_version).await?;

    tool.resolve_version(&version).await?;
    tool.locate_bins().await?;

    if use_shim {
        tool.setup_shims(true).await?;

        if let Some(shim_path) = tool.get_shim_path() {
            println!("{}", shim_path.to_string_lossy());

            return Ok(());
        }
    }

    println!("{}", tool.get_bin_path()?.to_string_lossy());

    Ok(())
}
