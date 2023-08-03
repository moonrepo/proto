use crate::shell::detect_shell;
use clap_complete::Shell;
use proto_core::ProtoEnvironment;
use starbase::SystemResult;
use std::env;
use std::path::PathBuf;
use tracing::debug;

pub async fn setup(shell: Option<Shell>, print_profile: bool) -> SystemResult {
    let shell = detect_shell(shell);
    let proto = ProtoEnvironment::new()?;

    let paths = env::var("PATH").expect("Missing PATH!");
    let paths = env::split_paths(&paths).collect::<Vec<_>>();

    if paths.contains(&proto.bin_dir) {
        debug!("Skipping setup, PROTO_ROOT already exists in PATH.");

        return Ok(());
    }

    do_setup(shell, proto.bin_dir, print_profile)?;

    Ok(())
}

// For other shells, write environment variable(s) to an applicable profile!
#[cfg(not(windows))]
fn do_setup(shell: Shell, _bin_dir: PathBuf, print_profile: bool) -> miette::Result<()> {
    use crate::shell::{format_env_vars, write_profile_if_not_setup};
    use rustc_hash::FxHashMap;

    debug!("Updating PATH in {} shell", shell);

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

// Windows does not support setting environment variables from a shell,
// so we're going to execute the `setx` command instead!
#[cfg(windows)]
fn do_setup(shell: Shell, bin_dir: PathBuf, print_profile: bool) -> miette::Result<()> {
    use std::process::Command;
    use tracing::warn;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let cu = RegKey::predef(HKEY_CURRENT_USER);

    let Ok(env) = cu.open_subkey("Environment") else {
        warn!("Failed to read current user environment");
        return Ok(());
    };

    let Ok(path) = env.get_value::<String, &str>("Path") else {
        warn!("Failed to read PATH from environment");
        return Ok(());
    };

    let cu_paths = env::split_paths(&path).collect::<Vec<_>>();

    if cu_paths.contains(&bin_dir) {
        return Ok(());
    }

    debug!(
        "Updating PATH with {} command",
        proto_core::color::shell("setx"),
    );

    let mut paths = vec![bin_dir];
    paths.extend(cu_paths);

    let mut command = Command::new("setx");
    command.arg("PATH");
    command.arg(env::join_paths(paths).unwrap());

    let output = command
        .output()
        .map_err(|e| ProtoError::Message(e.to_string()))?;

    if !output.status.success() {
        warn!("Failed to update PATH");
        debug!("[stderr]: {}", String::from_utf8_lossy(&output.stderr));
        debug!("[stdout]: {}", String::from_utf8_lossy(&output.stdout));
    } else if print_profile {
        println!("{}", shell);
    }

    Ok(())
}
