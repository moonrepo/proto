use crate::helpers::enable_logging;
use crate::shell::{detect_shell, format_env_vars, write_profile_if_not_setup};
use clap_complete::Shell;
use log::{debug, trace};
use proto_core::{color, get_root, ProtoError};
use rustc_hash::FxHashMap;
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub async fn setup(shell: Option<Shell>, print_profile: bool) -> Result<(), ProtoError> {
    let shell = detect_shell(shell);

    let Ok(paths) = env::var("PATH") else {
        return Err(ProtoError::MissingPathEnv);
    };

    enable_logging();

    let proto_dir = get_root()?;
    let bin_dir = proto_dir.join("bin");
    let paths = env::split_paths(&paths).collect::<Vec<_>>();

    if paths.contains(&bin_dir) {
        debug!(target: "proto:setup", "Skipping setup, PROTO_ROOT already exists in PATH.");

        return Ok(());
    }

    debug!(target: "proto:setup", "Updating PATH in {} shell", shell);

    // Windows does not support setting environment variables from a shell,
    // so we're going to execute the `setx` command instead!
    if cfg!(windows) {
        return setup_windows(bin_dir);
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

#[cfg(windows)]
fn setup_windows(bin_dir: PathBuf) -> Result<(), ProtoError> {
    use log::warn;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let cu = RegKey::predef(HKEY_CURRENT_USER);

    let Ok(env) = cu.open_subkey("Environment") else {
        warn!(target: "proto:setup", "Failed to read current user environment");
        return Ok(());
    };

    let Ok(path) = env.get_value::<String, &str>("Path") else {
        warn!(target: "proto:setup", "Failed to read PATH from environment");
        return Ok(());
    };

    let cu_paths = env::split_paths(&path).collect::<Vec<_>>();

    if cu_paths.contains(&bin_dir) {
        return Ok(());
    }

    debug!(target: "proto:setup", "Updating PATH with {} command", color::shell("setx"));

    let mut paths = vec![bin_dir];
    paths.extend(cu_paths);

    let mut command = Command::new("setx");
    command.arg("PATH");
    command.arg(env::join_paths(paths).unwrap());

    let output = command
        .output()
        .map_err(|e| ProtoError::Message(e.to_string()))?;

    if !output.status.success() {
        warn!(target: "proto:setup", "Failed to update PATH");

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
    }

    Ok(())
}
