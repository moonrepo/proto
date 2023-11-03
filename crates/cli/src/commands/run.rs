use crate::commands::install::{internal_install, InstallArgs};
use crate::error::ProtoCliError;
use clap::Args;
use miette::IntoDiagnostic;
use proto_core::{detect_version, load_tool, Id, ProtoError, Tool, UnresolvedVersionSpec};
use proto_pdk_api::RunHook;
use starbase::system;
use std::env;
use std::path::Path;
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

    #[arg(long, help = "Path to a relative alternate binary to run")]
    bin: Option<String>,

    // Passthrough args (after --)
    #[arg(
        last = true,
        help = "Arguments to pass through to the underlying command"
    )]
    passthrough: Vec<String>,
}

fn is_trying_to_self_upgrade(tool: &Tool, args: &[String]) -> bool {
    if tool.metadata.self_upgrade_commands.is_empty() {
        return false;
    }

    for arg in args {
        // Find first non-option arg
        if arg.starts_with('-') {
            continue;
        }

        // And then check if an upgrade command
        return tool.metadata.self_upgrade_commands.contains(arg);
    }

    false
}

fn create_command(exe_path: &Path, args: &[String]) -> Command {
    match exe_path.extension().map(|e| e.to_str().unwrap()) {
        Some("ps1" | "cmd" | "bat") => {
            let mut cmd = Command::new(if is_command_on_path("pwsh") {
                "pwsh"
            } else {
                "powershell"
            });
            cmd.arg("-Command");
            cmd.arg(format!(
                "{} {}",
                exe_path.display(),
                shell_words::join(args)
            ));
            cmd
        }
        _ => {
            let mut cmd = Command::new(exe_path);
            cmd.args(args);
            cmd
        }
    }
}

#[system]
pub async fn run(args: ArgsRef<RunArgs>) -> SystemResult {
    let mut tool = load_tool(&args.id).await?;

    // Avoid running the tool's native self-upgrade as it conflicts with proto
    if is_trying_to_self_upgrade(&tool, &args.passthrough) {
        return Err(ProtoCliError::NoSelfUpgrade {
            command: format!("proto install {} --pin", tool.id),
            tool: tool.get_name().to_owned(),
        }
        .into());
    }

    let version = detect_version(&tool, args.spec.clone()).await?;
    let user_config = tool.proto.load_user_config()?;

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

        tool = internal_install(
            InstallArgs {
                canary: false,
                id: args.id.clone(),
                pin: false,
                passthrough: vec![],
                spec: Some(tool.get_resolved_version().to_unresolved_spec()),
            },
            Some(tool),
        )
        .await?;
    }

    // Determine the binary path to execute
    let exe_path = match &args.bin {
        Some(alt_bin) => {
            let alt_path = tool.get_tool_dir().join(alt_bin);

            debug!(bin = alt_bin, path = ?alt_path, "Received an alternate binary to run with");

            if alt_path.exists() {
                alt_path
            } else {
                return Err(ProtoCliError::MissingRunAltBin {
                    bin: alt_bin.to_owned(),
                    path: alt_path,
                }
                .into());
            }
        }
        None => tool.get_exe_path()?.to_path_buf(),
    };

    debug!(bin = ?exe_path, args = ?args.passthrough, "Running {}", tool.get_name());

    // Run before hook
    tool.run_hook("pre_run", || RunHook {
        context: tool.create_context(),
        passthrough_args: args.passthrough.clone(),
    })?;

    // Run the command
    let status = create_command(&exe_path, &args.passthrough)
        .env(
            format!("{}_VERSION", tool.get_env_var_prefix()),
            tool.get_resolved_version().to_string(),
        )
        .env(
            format!("{}_BIN", tool.get_env_var_prefix()),
            exe_path.to_string_lossy().to_string(),
        )
        .spawn()
        .into_diagnostic()?
        .wait()
        .await
        .into_diagnostic()?;

    // Run after hook
    if status.success() {
        tool.run_hook("post_run", || RunHook {
            context: tool.create_context(),
            passthrough_args: args.passthrough.clone(),
        })?;
    }

    // Update the last used timestamp in a separate task,
    // as to not interrupt this task incase something fails!
    if env::var("PROTO_SKIP_USED_AT").is_err() {
        tokio::spawn(async move {
            tool.manifest.track_used_at(tool.get_resolved_version());
            let _ = tool.manifest.save();
        });
    }

    if !status.success() {
        exit(status.code().unwrap_or(1));
    }
}
