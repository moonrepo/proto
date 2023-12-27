use clap_complete::Shell;
use dirs::home_dir;
use proto_core::ENV_VAR;
use starbase_styles::color;
use starbase_utils::fs;
use std::{
    env,
    io::{self, BufRead},
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

    if let Ok(profile_env) = env::var("PROTO_SHELL_PROFILE") {
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
        Shell::PowerShell => {
            if let Ok(ps_home) = env::var("PSHOME") {
                let ps_home = PathBuf::from(ps_home);

                profiles.extend([
                    ps_home.join("Microsoft.PowerShell_profile.ps1"),
                    ps_home.join("Profile.ps1"),
                ]);
            }

            if cfg!(windows) {
                profiles.extend([
                    home_dir.join("Documents\\PowerShell\\Microsoft.PowerShell_profile.ps1"),
                    home_dir.join("Documents\\PowerShell\\Profile.ps1"),
                ]);
            } else {
                profiles.extend([
                    home_dir.join(".config/powershell/Microsoft.PowerShell_profile.ps1"),
                    home_dir.join(".config/powershell/profile.ps1"),
                ]);
            }
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

pub fn format_env_var(shell: &Shell, key: &str, value: &str) -> Option<String> {
    match shell {
        Shell::Bash | Shell::Zsh => Some(if key == "PATH" {
            format!(r#"export PATH="{value}:$PATH""#)
        } else {
            format!(r#"export {key}="{value}""#)
        }),
        Shell::Elvish => Some(if key == "PATH" {
            format!(
                "set paths [{} $@paths]",
                value
                    .replace(':', " ")
                    .replace("$HOME", "{~}")
                    .replace("$", "$E:")
            )
        } else {
            format!(
                "set-env {key} {}",
                value.replace("$HOME", "{~}").replace("$", "$E:")
            )
        }),
        // Shell::Elvish => {
        //     let value = if key == "PATH" {
        //         value.replace(':', " ")
        //     } else {
        //         value.to_owned()
        //     };
        //     let new_value = ENV_VAR.replace_all(&value, "$$E:$name");

        //     Some(if key == "PATH" {
        //         format!("set-env PATH (str:join ':' [{new_value} $E:PATH])")
        //     } else if key == "PROTO_HOME" {
        //         format!("set-env {key} {}", value.replace("$HOME", "{~}"))
        //     } else {
        //         format!("set-env {key} {new_value}")
        //     })
        // }
        Shell::Fish => Some(if key == "PATH" {
            format!(r#"set -gx PATH "{value}" $PATH"#)
        } else {
            format!(r#"set -gx {key} "{value}""#)
        }),
        Shell::PowerShell => {
            let value = if key == "PATH" {
                value.replace(':', ";")
            } else {
                value.to_owned()
            };
            let value = ENV_VAR.replace_all(&value, "$$env:$name");

            Some(if key == "PATH" {
                format!(r#"$env:PATH = "{value};" + $env:PATH"#)
            } else {
                format!(r#"$env:{key} = "{value}""#)
            })
        }
        _ => None,
    }
}

pub fn format_env_vars(
    shell: &Shell,
    comment: &str,
    vars: Vec<(String, String)>,
) -> Option<String> {
    let mut lines = vec![format!("\n# {comment}")];

    for (key, value) in vars {
        match format_env_var(shell, &key, &value) {
            Some(var) => lines.push(var),
            None => return None,
        };
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

    debug!(
        "Found no configured profile, updating {}",
        color::path(last_profile),
    );

    fs::append_file(last_profile, contents)?;

    debug!(
        "Setup profile {} with {}",
        color::path(last_profile),
        env_var,
    );

    Ok(Some(last_profile.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_env_vars() -> Vec<(String, String)> {
        vec![
            ("PROTO_HOME".to_string(), "$HOME/.proto".to_string()),
            (
                "PATH".to_string(),
                "$PROTO_HOME/shims:$PROTO_HOME/bin".to_string(),
            ),
        ]
    }

    #[test]
    fn formats_bash_env_vars() {
        assert_eq!(
            format_env_vars(&Shell::Bash, "Bash", get_env_vars()).unwrap(),
            r#"
# Bash
export PROTO_HOME="$HOME/.proto"
export PATH="$PROTO_HOME/shims:$PROTO_HOME/bin:$PATH""#
        );
    }

    #[test]
    fn formats_elvish_env_vars() {
        assert_eq!(
            format_env_vars(&Shell::Elvish, "Elvish", get_env_vars()).unwrap(),
            r#"
# Elvish
set-env PROTO_HOME {~}/.proto
set paths [$E:PROTO_HOME/shims $E:PROTO_HOME/bin $@paths]"#
        );
    }

    #[test]
    fn formats_fish_env_vars() {
        assert_eq!(
            format_env_vars(&Shell::Fish, "Fish", get_env_vars()).unwrap(),
            r#"
# Fish
set -gx PROTO_HOME "$HOME/.proto"
set -gx PATH "$PROTO_HOME/shims:$PROTO_HOME/bin" $PATH"#
        );
    }

    #[test]
    fn formats_pwsh_env_vars() {
        assert_eq!(
            format_env_vars(&Shell::PowerShell, "PowerShell", get_env_vars()).unwrap(),
            r#"
# PowerShell
$env:PROTO_HOME = "$env:HOME/.proto"
$env:PATH = "$env:PROTO_HOME/shims;$env:PROTO_HOME/bin;" + $env:PATH"#
        );
    }
}
