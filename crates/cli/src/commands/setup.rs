use crate::helpers::enable_logging;
use crate::shell::{find_profiles, write_profile_if_not_setup};
use clap_complete::Shell;
use log::{debug, trace};
use proto_core::{color, get_root, ProtoError};
use std::env;
use std::process::Command;

pub async fn setup(shell: Option<Shell>) -> Result<(), ProtoError> {
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

    debug!(target: "proto:setup", "Setting PATH in {} shell", shell);

    let proto_root = "$HOME/.proto";
    let content = match shell {
        Shell::Bash => {
            format!(
                r#"
# proto
export PROTO_ROOT="{proto_root}"
export PATH="$PROTO_ROOT/bin:$PATH""#,
            )
        }
        Shell::Elvish => {
            format!(
                r#"
# proto
set-env PROTO_ROOT {proto_root}
set-env PATH (str:join ':' [$PROTO_ROOT/bin $E:PATH])"#
            )
        }
        Shell::Fish => {
            format!(
                r#"
# proto
set -gx PROTO_ROOT "{proto_root}"
set -gx PATH "$PROTO_ROOT/bin" $PATH"#
            )
        }
        Shell::Zsh => {
            format!(
                r#"
# proto
export PROTO_ROOT="{proto_root}"
export PATH="$PROTO_ROOT/bin:$PATH""#
            )
        }
        // Windows does not support setting environment variables from a shell,
        // so we're going to execute the `setx` command instead!
        Shell::PowerShell => {
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
        _ => {
            return Ok(());
        }
    };

    let profiles = find_profiles(&shell)?;

    write_profile_if_not_setup(&profiles, content, "PROTO_ROOT")?;

    Ok(())
}
