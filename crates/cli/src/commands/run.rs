use crate::commands::install::install;
use crate::hooks::node as node_hooks;
use crate::tools::create_tool;
use miette::IntoDiagnostic;
use proto_core::{detect_version, AliasOrVersion, ProtoError, UserConfig, VersionType};
use starbase::SystemResult;
use starbase_styles::color;
use std::env;
use std::process::exit;
use tokio::process::Command;
use tracing::debug;


pub async fn run(
    tool_id: String,
    forced_version: Option<VersionType>,
    alt_bin: Option<String>,
    args: Vec<String>,
) -> SystemResult {
    let mut tool = create_tool(&tool_id).await?;
    let version = detect_version(&tool, forced_version).await?;
    let user_config = UserConfig::load()?;

    if !tool.is_setup(&version).await? {
        if !user_config.auto_install {
            return Err(ProtoError::MissingToolForRun {
                tool: tool.get_name(),
                version: version.to_string(),
                command: format!("proto install {} {}", tool.id, tool.get_resolved_version()),
            }
            .into());
        }

        // Install the tool
        debug!("Auto-install setting is configured, attempting to install");

        install(
            tool_id.clone(),
            Some(tool.get_resolved_version().to_implicit_type()),
            false,
            vec![],
        )
        .await?;

        // Find the new binaries
        tool.locate_bins().await?;
    }

    let resolved_version = tool.get_resolved_version();

    // Update the last used timestamp
    if env::var("PROTO_SKIP_USED_AT").is_err() {
        if let AliasOrVersion::Version(version) = &resolved_version {
            tool.manifest.track_used_at(version);

            // Ignore errors in case of race conditions...
            // this timestamp isn't *super* important
            let _ = tool.manifest.save();
        }
    }

    // Determine the binary path
    let tool_dir = tool.get_tool_dir();
    let mut bin_path = tool.get_bin_path()?.to_path_buf();

    if let Some(alt_bin) = alt_bin {
        let alt_bin_path = tool_dir.join(&alt_bin);

        debug!(bin = alt_bin, path = ?alt_bin_path, "Received an alternate binary to run with");

        if alt_bin_path.exists() {
            bin_path = alt_bin_path;
        } else {
            return Err(ProtoError::Message(format!(
                "Alternate binary {} does not exist.",
                color::file(&alt_bin)
            ))
            .into());
        }
    } else if let Some(shim_path) = tool.get_shim_path() {
        bin_path = shim_path;

        debug!(shim = ?bin_path, "Using local shim for tool");
    }

    debug!(bin = ?bin_path, "Running {}", tool.get_name());

    // Trigger before hook
    if tool_id == "npm" || tool_id == "pnpm" || tool_id == "yarn" {
        node_hooks::pre_run(&tool_id, &args, &user_config).await?;
    }

    // Run the command
    let mut command = match bin_path.extension().map(|e| e.to_str().unwrap()) {
        Some("ps1") => {
            let mut cmd = Command::new("powershell");
            cmd.arg("--File").arg(bin_path);
            cmd
        }
        Some("cmd" | "bat") => {
            let mut cmd = Command::new("cmd");
            cmd.arg("/q").arg("/c").arg(bin_path);
            cmd
        }
        _ =>  Command::new(bin_path),
    };

    let status = command
        .args(&args)
        .env(
            format!("{}_VERSION", tool.get_env_var_prefix()),
            resolved_version.to_string(),
        )
        .env(
            format!("{}_BIN", tool.get_env_var_prefix()),
            tool.get_bin_path()?.to_string_lossy().to_string(),
        )
        .spawn()
        .into_diagnostic()?
        .wait()
        .await
        .into_diagnostic()?;

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }

    Ok(())
}
