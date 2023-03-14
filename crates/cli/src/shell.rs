use clap_complete::Shell;
use dirs::home_dir;
use log::{debug, trace};
use proto_core::{color, ProtoError};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, BufRead, Write},
    path::PathBuf,
};

pub fn find_profiles(shell: &Shell) -> Result<Vec<PathBuf>, ProtoError> {
    debug!(target: "proto:shell", "Finding profile files for {}", shell);

    let home_dir = home_dir().expect("Invalid home directory.");
    let mut profiles = vec![home_dir.join(".profile")];

    if let Ok(profile_env) = env::var("PROFILE") {
        if !profile_env.is_empty() {
            profiles.push(PathBuf::from(profile_env));
        }
    }

    match shell {
        Shell::Bash => {
            profiles.extend([home_dir.join(".bash_profile"), home_dir.join(".bashrc")]);
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
        }
        Shell::Fish => {
            profiles.push(home_dir.join(".config/fish/config.fish"));
        }
        Shell::Zsh => {
            let zdot_dir = if let Ok(dir) = env::var("ZDOTDIR") {
                PathBuf::from(dir)
            } else {
                home_dir
            };

            profiles.extend([zdot_dir.join(".zprofile"), zdot_dir.join(".zshrc")]);
        }
        _ => {}
    };

    Ok(profiles)
}

pub fn write_profile_if_not_setup(
    profiles: &[PathBuf],
    contents: String,
    env_var: &str,
) -> Result<Option<PathBuf>, ProtoError> {
    for profile in profiles {
        trace!(target: "proto:shell", "Checking if shell profile {} exists", color::path(profile));

        if !profile.exists() {
            trace!(target: "proto:shell", "Not found, continuing");
            continue;
        }

        trace!(target: "proto:shell", "Exists, checking if already setup");

        let file = fs::File::open(profile)
            .map_err(|e| ProtoError::Fs(profile.to_path_buf(), e.to_string()))?;

        let has_setup = io::BufReader::new(file)
            .lines()
            .map(|l| l.unwrap_or_default())
            .any(|l| l.contains(env_var));

        // Already setup profile, so avoid writing
        if has_setup {
            debug!(
                target: "proto:shell",
                "Profile {} already setup for {}",
                color::path(profile),
                env_var,
            );

            return Ok(None);
        }

        trace!(target: "proto:shell", "Not setup, continuing");
    }

    // Create a profile if none found. Use the last profile in the list
    // as it's the "most common", and is typically the interactive shell.
    let last_profile = profiles.last().unwrap();
    let handle_error = |e: io::Error| ProtoError::Fs(last_profile.to_path_buf(), e.to_string());

    debug!(
        target: "proto:shell",
        "Found no configured profile, updating {}",
        color::path(last_profile),
    );

    if let Some(parent) = last_profile.parent() {
        fs::create_dir_all(parent).map_err(handle_error)?;
    }

    let mut options = OpenOptions::new();
    options.read(true);
    options.append(true);
    options.create(true);

    let mut file = options.open(last_profile).map_err(handle_error)?;

    write!(file, "{contents}").map_err(handle_error)?;

    debug!(
        target: "proto:shell",
        "Setup profile {} with {}",
        color::path(last_profile),
        env_var,
    );

    Ok(Some(last_profile.to_path_buf()))
}
