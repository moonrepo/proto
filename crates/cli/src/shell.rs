use clap_complete::Shell;
use dirs::home_dir;
use rustc_hash::FxHashMap;
use starbase_styles::color;
use starbase_utils::fs::{self, FsError};
use std::{
    env,
    fs::OpenOptions,
    io::{self, BufRead, Write},
    path::PathBuf,
};
use tracing::debug;

pub fn detect_shell(shell: Option<Shell>) -> Shell {
    shell.or_else(Shell::from_env).unwrap_or({
        if cfg!(windows) {
            Shell::PowerShell
        } else {
            Shell::Bash
        }
    })
}

pub fn find_profiles(shell: &Shell) -> miette::Result<Vec<PathBuf>> {
    debug!("Finding profile files for {}", shell);

    if let Ok(profile_env) = env::var("TEST_PROFILE") {
        return Ok(vec![PathBuf::from(profile_env)]);
    }

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

pub fn format_env_vars(
    shell: &Shell,
    comment: &str,
    vars: FxHashMap<String, String>,
) -> Option<String> {
    let mut lines = vec![format!("\n# {comment}")];

    for (key, value) in vars {
        match shell {
            Shell::Bash | Shell::Zsh => {
                if key == "PATH" {
                    lines.push(format!(r#"export PATH="{value}:$PATH""#));
                } else {
                    lines.push(format!(r#"export {key}="{value}""#));
                }
            }
            Shell::Elvish => {
                if key == "PATH" {
                    lines.push(format!(r#"set-env PATH (str:join ':' [{value} $E:PATH])"#));
                } else {
                    lines.push(format!(r#"set-env {key} {value}"#));
                }
            }
            Shell::Fish => {
                if key == "PATH" {
                    lines.push(format!(r#"set -gx PATH "{value}" $PATH"#));
                } else {
                    lines.push(format!(r#"set -gx {key} "{value}""#));
                }
            }
            _ => return None,
        }
    }

    Some(lines.join("\n"))
}

pub fn write_profile_if_not_setup(
    shell: &Shell,
    contents: String,
    env_var: &str,
) -> miette::Result<Option<PathBuf>> {
    let profiles = find_profiles(shell)?;

    for profile in &profiles {
        debug!("Checking if shell profile {} exists", color::path(profile));

        if !profile.exists() {
            debug!("Not found, continuing");
            continue;
        }

        debug!("Exists, checking if already setup");

        let file = fs::open_file(profile)?;

        let has_setup = io::BufReader::new(file)
            .lines()
            .map(|l| l.unwrap_or_default())
            .any(|l| l.contains(env_var));

        // Already setup profile, so avoid writing
        if has_setup {
            debug!(
                "Profile {} already setup for {}",
                color::path(profile),
                env_var,
            );

            return Ok(None);
        }

        debug!("Not setup, continuing");
    }

    // Create a profile if none found. Use the last profile in the list
    // as it's the "most common", and is typically the interactive shell.
    let last_profile = profiles.last().unwrap();
    let handle_error = |error: io::Error| FsError::Write {
        path: last_profile.to_path_buf(),
        error,
    };

    debug!(
        "Found no configured profile, updating {}",
        color::path(last_profile),
    );

    if let Some(parent) = last_profile.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut options = OpenOptions::new();
    options.read(true);
    options.append(true);
    options.create(true);

    let mut file = options.open(last_profile).map_err(handle_error)?;

    writeln!(file, "{contents}").map_err(handle_error)?;

    debug!(
        "Setup profile {} with {}",
        color::path(last_profile),
        env_var,
    );

    Ok(Some(last_profile.to_path_buf()))
}
