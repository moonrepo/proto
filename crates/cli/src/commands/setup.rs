use crate::helpers::enable_logging;
use clap_complete::Shell;
use dirs::home_dir;
use log::{debug, info, trace};
use proto::{color, get_root, ProtoError};
use std::io::{self, BufRead, Write};
use std::process::Command;
use std::{env, fs, path::PathBuf};

fn write_profile(shell: &Shell, profiles: &[PathBuf], contents: String) -> Result<(), ProtoError> {
    for profile in profiles {
        trace!(target: "proto:setup", "Checking if profile {} exists", color::path(&profile));

        if !profile.exists() {
            continue;
        }

        trace!(target: "proto:setup", "Exists, checking if proto already setup");

        let profile_path = if profile.is_symlink() {
            let original = fs::canonicalize(profile).unwrap();

            trace!(
                target: "proto:setup",
                "Found a symlink, resolved to {}",
                color::path(&original),
            );

            original
        } else {
            profile.to_path_buf()
        };

        let file = fs::File::open(profile_path)
            .map_err(|e| ProtoError::Fs(profile.to_path_buf(), e.to_string()))?;

        let has_setup = io::BufReader::new(file)
            .lines()
            .map(|l| l.unwrap_or_default())
            .any(|l| l.contains("# proto") || l.contains("PROTO_ROOT"));

        // proto has already been setup in a profile, so avoid writing
        if has_setup {
            info!(target: "proto:setup", "proto has already been setup for {}", shell);

            return Ok(());
        }
    }

    // Create a profile if none found. Use the last profile in the list
    // as it's the "most common", and is typically the interactive shell.
    let last_profile = profiles.last().unwrap();
    let handle_error = |e: io::Error| ProtoError::Fs(last_profile.to_path_buf(), e.to_string());

    trace!(
        target: "proto:setup",
        "Found no profile, create and writing PATH to {}",
        color::path(&last_profile),
    );

    let mut file = fs::File::create(last_profile).map_err(handle_error)?;

    write!(file, "{}", contents).map_err(handle_error)?;

    info!(target: "proto:setup", "Setup proto for {}", shell);

    Ok(())
}

pub async fn setup(shell: Option<Shell>) -> Result<(), ProtoError> {
    let Some(shell) = shell.or_else(Shell::from_env) else {
        return Err(ProtoError::UnsupportedShell);
    };

    let Ok(paths) = env::var("PATH") else {
        return Err(ProtoError::MissingPathEnv);
    };

    enable_logging();

    let home_dir = home_dir().expect("Invalid home directory.");
    let proto_dir = get_root()?;
    let mut paths = env::split_paths(&paths).collect::<Vec<_>>();

    if paths.iter().any(|p| p == &proto_dir) {
        println!("Skipping setup, PROTO_ROOT already exists in PATH.");

        return Ok(());
    }

    debug!(target: "proto:setup", "Setting PATH in {} shell", shell);

    let proto_root = "$HOME/.proto";
    let mut profiles = vec![home_dir.join(".profile")];

    if let Ok(profile_env) = env::var("PROFILE") {
        if !profile_env.is_empty() {
            profiles.push(PathBuf::from(profile_env));
        }
    }

    match shell {
        Shell::Bash => {
            profiles.extend([home_dir.join(".bash_profile"), home_dir.join(".bashrc")]);

            write_profile(
                &shell,
                &profiles,
                format!(
                    r#"
# proto
export PROTO_ROOT="{}"
export PATH="$PROTO_ROOT/shims:$PATH""#,
                    proto_root
                ),
            )?;
        }
        Shell::Elvish => {
            profiles.push(home_dir.join(".elvish/rc.elv"));

            if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
                profiles.push(PathBuf::from(xdg_config).join("elvish/rc.elv"));
            }

            if let Ok(app_data) = env::var("AppData") {
                profiles.push(PathBuf::from(app_data).join("elvish/rc.elv"));
            } else {
                profiles.push(home_dir.join(".config/elvish/rc.elv"));
            }

            write_profile(
                &shell,
                &profiles,
                format!(
                    r#"
# proto
set-env PROTO_ROOT {}
set-env PATH (str:join ':' [$E:PATH $PROTO_ROOT/shims])"#,
                    proto_root
                ),
            )?;
        }
        Shell::Fish => {
            profiles.push(home_dir.join(".config/fish/config.fish"));

            write_profile(
                &shell,
                &profiles,
                format!(
                    r#"
# proto
set -gx PROTO_ROOT "{}"
set -gx PATH "$PROTO_ROOT/shims" $PATH"#,
                    proto_root
                ),
            )?;
        }
        Shell::Zsh => {
            let zdot_dir = if let Ok(dir) = env::var("ZDOTDIR") {
                PathBuf::from(dir)
            } else {
                home_dir
            };

            profiles.extend([zdot_dir.join(".zprofile"), zdot_dir.join(".zshrc")]);

            write_profile(
                &shell,
                &profiles,
                format!(
                    r#"
# proto
export PROTO_ROOT="{}"
export PATH="$PROTO_ROOT/shims:$PATH""#,
                    proto_root
                ),
            )?;
        }
        // Windows does not support setting environment variables from a shell,
        // so we're going to execute the `setx` command instead!
        Shell::PowerShell => {
            paths.push(proto_dir.join("shims"));

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
        }
        _ => {}
    };

    Ok(())
}
