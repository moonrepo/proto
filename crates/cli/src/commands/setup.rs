use clap_complete::Shell;
use dirs::home_dir;
use proto::{get_root, ProtoError};
use std::io::{self, BufRead, Write};
use std::{env, fs, path::PathBuf};

fn write_profile(profiles: &[PathBuf], contents: String) -> Result<(), ProtoError> {
    for profile in profiles {
        if !profile.exists() {
            continue;
        }

        let file = fs::File::open(profile)
            .map_err(|e| ProtoError::Fs(profile.to_path_buf(), e.to_string()))?;

        let has_setup = io::BufReader::new(file)
            .lines()
            .map(|l| l.unwrap_or_default())
            .any(|l| l.contains("# proto") || l.contains("PROTO_ROOT"));

        // proto has already been setup in a profile, so avoid writing
        if has_setup {
            return Ok(());
        }
    }

    // Create a profile if none found. Use the last profile in the list
    // as it's the "most common", and is typically the interactive shell.
    let last_profile = profiles.last().unwrap();
    let handle_error = |e: io::Error| ProtoError::Fs(last_profile.to_path_buf(), e.to_string());

    let mut file = fs::File::create(last_profile).map_err(handle_error)?;

    write!(file, "{}", contents).map_err(handle_error)?;

    Ok(())
}

pub async fn setup(shell: Option<Shell>) -> Result<(), ProtoError> {
    let Some(shell) = shell.or_else(Shell::from_env) else {
      return Err(ProtoError::UnsupportedShell);
    };

    let home_dir = home_dir().expect("Invalid home directory.");
    let proto_dir = get_root()?;

    if let Ok(paths) = env::var("PATH") {
        if env::split_paths(&paths).any(|p| p == proto_dir) {
            println!("Skipping setup, PROTO_ROOT already exists in PATH.");

            return Ok(());
        }
    }

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
                &profiles,
                format!(
                    r#"
# proto
export PROTO_ROOT="{}"
export PATH="$PROTO_ROOT/shims:$PATH"
                    "#,
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
                &profiles,
                format!(
                    r#"
# proto
set-env PROTO_ROOT {}
set-env PATH (str:join ':' [$E:PATH $PROTO_ROOT/shims])
                    "#,
                    proto_root
                ),
            )?;
        }
        Shell::Fish => {
            profiles.push(home_dir.join(".config/fish/config.fish"));

            write_profile(
                &profiles,
                format!(
                    r#"
# proto
set -gx PROTO_ROOT "{}"
set -gx PATH "$PROTO_ROOT/shims" $PATH
                    "#,
                    proto_root
                ),
            )?;
        }
        Shell::PowerShell => {
            // TODO
        }
        Shell::Zsh => {
            let zdot_dir = if let Ok(dir) = env::var("ZDOTDIR") {
                PathBuf::from(dir)
            } else {
                home_dir
            };

            profiles.extend([zdot_dir.join(".zprofile"), zdot_dir.join(".zshrc")]);

            write_profile(
                &profiles,
                format!(
                    r#"
# proto
export PROTO_ROOT="{}"
export PATH="$PROTO_ROOT/shims:$PATH"
                    "#,
                    proto_root
                ),
            )?;
        }
        _ => {}
    };

    Ok(())
}
