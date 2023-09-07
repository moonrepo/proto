use crate::commands::install::{internal_install, InstallArgs};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{detect_version, load_tool, Id, ProtoError, UserConfig, VersionType};
use proto_pdk_api::RunHook;
use starbase::system;
use starbase_styles::color;
use std::env;
use std::process::exit;
use tokio::process::Command;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(help = "Version or alias of tool")]
    semver: Option<VersionType>,

    #[arg(long, help = "Path to an alternate binary to run")]
    bin: Option<String>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "Arguments to pass through to the underlying command"
    )]
    passthrough: Vec<String>,
}

#[system]
pub async fn run(args: ArgsRef<RunArgs>) -> SystemResult {
    let mut tool = load_tool(&args.id).await?;
    let version = detect_version(&tool, args.semver.clone()).await?;
    let user_config = UserConfig::load()?;

    // Check if installed or install
    if !tool.is_setup(&version).await? {
        if !user_config.auto_install {
            return Err(ProtoError::MissingToolForRun {
                tool: tool.get_name().to_owned(),
                version: version.to_string(),
                command: format!("proto install {} {}", tool.id, tool.get_resolved_version()),
            }
            .into());
        }

        // Install the tool
        debug!("Auto-install setting is configured, attempting to install");

        internal_install(InstallArgs {
            id: args.id.clone(),
            semver: Some(tool.get_resolved_version().to_implicit_type()),
            pin: false,
            passthrough: vec![],
        })
        .await?;

        // Find the new binaries
        tool.locate_bins().await?;
    }

    let resolved_version = tool.get_resolved_version();

    // Update the last used timestamp
    if env::var("PROTO_SKIP_USED_AT").is_err() {
        tool.manifest.track_used_at(&resolved_version);

        // Ignore errors in case of race conditions...
        // this timestamp isn't *super* important
        let _ = tool.manifest.save();
    }

    // Determine the binary path to execute
    let tool_dir = tool.get_tool_dir();
    let mut bin_path = tool.get_bin_path()?.to_path_buf();

    if let Some(alt_bin) = &args.bin {
        let alt_bin_path = tool_dir.join(alt_bin);

        debug!(bin = alt_bin, path = ?alt_bin_path, "Received an alternate binary to run with");

        if alt_bin_path.exists() {
            bin_path = alt_bin_path;
        } else {
            return Err(ProtoError::Message(format!(
                "Alternate binary {} does not exist.",
                color::file(alt_bin)
            ))
            .into());
        }
    } else if let Some(shim_path) = tool.get_shim_path() {
        bin_path = shim_path;

        debug!(shim = ?bin_path, "Using local shim for tool");
    }

    debug!(bin = ?bin_path, args = ?args, "Running {}", tool.get_name());

    // Run before hook
    tool.run_hook(
        "pre_run",
        RunHook {
            context: tool.create_context()?,
            passthrough_args: args.passthrough.clone(),
        },
    )?;

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
        _ => Command::new(bin_path),
    };

    let status = command
        .args(&args.passthrough)
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

    // Run after hook
    tool.run_hook(
        "post_run",
        RunHook {
            context: tool.create_context()?,
            passthrough_args: args.passthrough.clone(),
        },
    )?;
}
