use crate::shell::detect_shell;
use clap::Args;
use clap_complete::Shell;
use proto_core::ProtoEnvironment;
use starbase::system;
use std::env;
use std::path::PathBuf;
use tracing::debug;

#[derive(Args, Clone, Debug)]
pub struct SetupArgs {
    #[arg(long, help = "Shell to setup for")]
    shell: Option<Shell>,

    #[arg(long, help = "Return the profile path if setup")]
    profile: bool,
}

#[system]
pub async fn setup(args: ArgsRef<SetupArgs>) {
    let shell = detect_shell(args.shell);
    let proto = ProtoEnvironment::new()?;

    let paths = env::var("PATH").expect("Missing PATH!");
    let paths = env::split_paths(&paths).collect::<Vec<_>>();

    if paths.contains(&proto.shims_dir) || paths.contains(&proto.bin_dir) {
        debug!("Skipping setup, PROTO_HOME already exists in PATH.");

        return Ok(());
    }

    do_setup(shell, vec![proto.shims_dir, proto.bin_dir], args.profile)?;
}

// For other shells, write environment variable(s) to an applicable profile!
#[cfg(not(windows))]
fn do_setup(shell: Shell, _dirs: Vec<PathBuf>, print_profile: bool) -> miette::Result<()> {
    use crate::shell::{format_env_vars, write_profile_if_not_setup};

    debug!("Updating PATH in {} shell", shell);

    let env_vars = vec![
        ("PROTO_HOME".to_string(), "$HOME/.proto".to_string()),
        (
            "PATH".to_string(),
            "$PROTO_HOME/shims:$PROTO_HOME/bin".to_string(),
        ),
    ];

    if let Some(content) = format_env_vars(&shell, "proto", env_vars) {
        if let Some(updated_profile) = write_profile_if_not_setup(&shell, content, "PROTO_HOME")? {
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
fn do_setup(shell: Shell, mut dirs: Vec<PathBuf>, print_profile: bool) -> miette::Result<()> {
    use miette::IntoDiagnostic;
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

    if cu_paths.contains(&dirs[0]) || cu_paths.contains(&dirs[1]) {
        return Ok(());
    }

    debug!(
        "Updating PATH with {} command",
        starbase_styles::color::shell("setx"),
    );

    dirs.extend(cu_paths);

    let mut command = Command::new("setx");
    command.arg("PATH");
    command.arg(env::join_paths(dirs).unwrap());

    let output = command.output().into_diagnostic()?;

    if !output.status.success() {
        warn!("Failed to update PATH");
        debug!("[stderr]: {}", String::from_utf8_lossy(&output.stderr));
        debug!("[stdout]: {}", String::from_utf8_lossy(&output.stdout));
    } else if print_profile {
        println!("{}", shell);
    }

    Ok(())
}
