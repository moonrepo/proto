use crate::commands::install::{internal_install, InstallArgs};
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{detect_version, load_tool, Id, ProtoError, UnresolvedVersionSpec};
use proto_pdk_api::RunHook;
use starbase::system;
use starbase_styles::color;
use std::env;
use std::process::exit;
use system_env::is_command_on_path;
use tokio::process::Command;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(required = true, help = "ID of tool")]
    id: Id,

    #[arg(help = "Version or alias of tool")]
    spec: Option<UnresolvedVersionSpec>,

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
    let version = detect_version(&tool, args.spec.clone()).await?;
    let user_config = tool.proto.get_user_config()?;

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
            canary: false,
            id: args.id.clone(),
            pin: false,
            passthrough: vec![],
            spec: Some(tool.get_resolved_version().to_unresolved_spec()),
        })
        .await?;

        // Find the new binaries
        tool.locate_bins().await?;
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

    debug!(bin = ?bin_path, args = ?args.passthrough, "Running {}", tool.get_name());

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
            let mut cmd = Command::new(if is_command_on_path("pwsh") {
                "pwsh"
            } else {
                "powershell"
            });
            cmd.arg("-Command").arg(format!(
                "{} {}",
                bin_path.display(),
                args.passthrough.join(" ")
            ));
            cmd
        }
        Some("cmd" | "bat") => {
            let mut cmd = Command::new("cmd");
            cmd.arg("/q").arg("/c").arg(format!(
                "{} {}",
                bin_path.display(),
                args.passthrough.join(" ")
            ));
            cmd
        }
        _ => {
            let mut cmd = Command::new(bin_path);
            cmd.args(&args.passthrough);
            cmd
        }
    };

    let status = command
        .env(
            format!("{}_VERSION", tool.get_env_var_prefix()),
            tool.get_resolved_version().to_string(),
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

    // Update the last used timestamp in a separate task,
    // as to not interrupt this task incase something fails!
    if env::var("PROTO_SKIP_USED_AT").is_err() {
        let mut manifest = tool.manifest.clone();
        let version = tool.get_resolved_version();

        tokio::spawn(async move {
            manifest.track_used_at(version);
            let _ = manifest.save();
        });
    }

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
