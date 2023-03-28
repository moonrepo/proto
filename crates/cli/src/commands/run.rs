use crate::commands::install::install;
use crate::tools::{create_tool, ToolType};
use log::debug;
use proto::UserConfig;
use proto_core::{detect_version_from_environment, ProtoError};
use std::process::exit;
use tokio::process::Command;

pub async fn run(
    tool_type: ToolType,
    forced_version: Option<String>,
    args: Vec<String>,
) -> Result<(), ProtoError> {
    let mut tool = create_tool(&tool_type)?;
    let version = detect_version_from_environment(&tool, forced_version).await?;

    if !tool.is_setup(&version).await? {
        let config = UserConfig::load()?;

        if !config.auto_install {
            return Err(ProtoError::MissingToolForRun(
                tool.get_name(),
                version.to_owned(),
                format!("proto install {} {}", tool.get_bin_name(), version),
            ));
        }

        // Install the tool
        debug!(
            target: tool.get_log_target(),
            "Auto-install setting is configured, attempting to install"
        );

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

    let status = Command::new(tool.get_bin_path()?)
        .args(&args)
        .env(
            format!("PROTO_{}_VERSION", tool.get_bin_name().to_uppercase()),
            tool.get_resolved_version(),
        )
        .env(
            format!("PROTO_{}_BIN", tool.get_bin_name().to_uppercase()),
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
