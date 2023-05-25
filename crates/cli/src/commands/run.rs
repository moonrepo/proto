use crate::commands::install::install;
use crate::states::UserConfig;
use crate::tools::{create_tool, ToolType};
use proto_core::{color, detect_version, ProtoError};
use starbase::SystemResult;
use std::env;
use std::process::exit;
use tokio::process::Command;
use tracing::debug;

pub async fn run(
    tool_type: ToolType,
    forced_version: Option<String>,
    alt_bin: Option<String>,
    args: Vec<String>,
    user_config: &UserConfig,
) -> SystemResult {
    let mut tool = create_tool(&tool_type).await?;
    let version = detect_version(&tool, forced_version).await?;

    if !tool.is_setup(&version).await? {
        if !user_config.auto_install {
            return Err(ProtoError::MissingToolForRun(
                tool.get_name(),
                version.to_owned(),
                color::shell(format!("proto install {} {}", tool.get_id(), version)),
            ))?;
        }

        // Install the tool
        debug!("Auto-install setting is configured, attempting to install");

        install(
            tool_type,
            Some(tool.get_resolved_version().to_owned()),
            false,
            vec![],
        )
        .await?;

        // Find the new binaries
        tool.find_bin_path().await?;
    }

    let resolved_version = tool.get_resolved_version().to_owned();

    // Update the last used timestamp
    if env::var("PROTO_SKIP_USED_AT").is_err() {
        let manifest = tool.get_manifest_mut()?;
        manifest.track_used_at(&resolved_version);

        // Ignore errors in case of race conditions...
        // this timestamp isn't *super* important
        let _ = manifest.save();
    }

    // Determine the binary path
    let mut bin_path = tool.get_bin_path()?.to_path_buf();

    if let Some(alt_bin) = alt_bin {
        let alt_bin_path = bin_path.parent().unwrap().join(&alt_bin);

        if alt_bin_path.exists() {
            bin_path = alt_bin_path;

            debug!(alt_bin, "Received an alternate binary to run with");
        } else {
            return Err(ProtoError::Message(format!(
                "Alternate binary {} does not exist.",
                color::file(&alt_bin)
            )))?;
        }
    }

    debug!(bin = %bin_path.display(), "Running {}", tool.get_name());

    // Run the command
    let status = Command::new(bin_path)
        .args(&args)
        .env(
            format!("PROTO_{}_VERSION", tool.get_id().to_uppercase()),
            tool.get_resolved_version(),
        )
        .env(
            format!("PROTO_{}_BIN", tool.get_id().to_uppercase()),
            tool.get_bin_path()?.to_string_lossy().to_string(),
        )
        .spawn()
        .map_err(|e| ProtoError::Message(e.to_string()))?
        .wait()
        .await
        .map_err(|e| ProtoError::Message(e.to_string()))?;

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}
