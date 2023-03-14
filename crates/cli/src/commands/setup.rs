use crate::helpers::enable_logging;
use crate::shell::{format_env_vars, write_profile_if_not_setup};
use clap_complete::Shell;
use log::{debug, trace};
use proto_core::{color, get_root, ProtoError};
use rustc_hash::FxHashMap;
use std::env;
use std::process::Command;

pub async fn setup(shell: Option<Shell>, print_profile: bool) -> Result<(), ProtoError> {
    let Some(shell) = shell.or_else(Shell::from_env) else {
        return Err(ProtoError::UnsupportedShell);
    };

    let Ok(paths) = env::var("PATH") else {
        return Err(ProtoError::MissingPathEnv);
    };

    enable_logging();

    let proto_dir = get_root()?;
    let mut paths = env::split_paths(&paths).collect::<Vec<_>>();

    if paths.iter().any(|p| p == &proto_dir) {
        debug!(target: "proto:setup", "Skipping setup, PROTO_ROOT already exists in PATH.");

        return Ok(());
    }

    debug!(target: "proto:setup", "Updating PATH in {} shell", shell);

    // Windows does not support setting environment variables from a shell,
    // so we're going to execute the `setx` command instead!
    if matches!(shell, Shell::PowerShell) {
        paths.push(proto_dir.join("bin"));

        debug!(target: "proto:setup", "Using {} command", color::shell("setx"));

        let mut command = Command::new("setx");
        command.arg("PATH");
        command.arg(env::join_paths(paths).unwrap());

        let output = command
            .output()
            .map_err(|e| ProtoError::Message(e.to_string()))?;

        if !output.status.success() {
            trace!(
                target: "proto:setup",
                "STDERR: {}",
                String::from_utf8_lossy(&output.stderr),
            );

            trace!(
                target: "proto:setup",
                "STDOUT: {}",
                String::from_utf8_lossy(&output.stdout),
            );

            return Err(ProtoError::WritePathFailed);
        }

        return Ok(());
    }

    // For other shells, write environment variable(s) to an applicable profile!
    let env_vars = FxHashMap::from_iter([
        ("PROTO_ROOT".to_string(), "$HOME/.proto".to_string()),
        ("PATH".to_string(), "$PROTO_ROOT/bin".to_string()),
    ]);

    if let Some(content) = format_env_vars(&shell, "proto", env_vars) {
        if let Some(updated_profile) = write_profile_if_not_setup(&shell, content, "PROTO_ROOT")? {
            if print_profile {
                println!("{}", updated_profile.to_string_lossy());
            }
        }
    }

    Ok(())
}
