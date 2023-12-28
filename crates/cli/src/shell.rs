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

pub enum Export {
    Path(Vec<String>),
    Var(String, String),
}

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

pub fn format_export(shell: &Shell, var: Export) -> Option<String> {
    let result = match shell {
        Shell::Bash | Shell::Zsh => match var {
            Export::Path(paths) => format!(r#"export PATH="{}:$PATH""#, paths.join(":")),
            Export::Var(key, value) => format!(r#"export {key}="{value}""#),
        },
        Shell::Elvish => {
            fn format(value: String) -> String {
                ENV_VAR
                    .replace_all(&value, "$$E:$name")
                    .replace("$E:HOME", "{~}")
            }

            match var {
                Export::Path(paths) => format!("set paths [{} $@paths]", format(paths.join(" "))),
                Export::Var(key, value) => format!("set-env {key} {}", format(value)),
            }
        }
        Shell::Fish => match var {
            Export::Path(paths) => format!(r#"set -gx PATH "{}" $PATH"#, paths.join(":")),
            Export::Var(key, value) => format!(r#"set -gx {key} "{value}""#),
        },
        Shell::PowerShell => {
            fn format(value: String) -> String {
                ENV_VAR
                    .replace_all(&value, "$$env:$name")
                    .replace("$env:HOME", "$HOME")
            }

            fn join_path(value: String) -> String {
                let parts = value
                    .split("/")
                    .map(|part| {
                        if part.starts_with("$") {
                            part.to_owned()
                        } else {
                            format!("\"{}\"", part)
                        }
                    })
                    .collect::<Vec<_>>();

                format(format!("Join-Path {}", parts.join(" ")))
            }

            match var {
                Export::Path(paths) => {
                    let mut value = "$env:PATH = @(\n".to_owned();

                    for path in paths {
                        value.push_str(&format!("  ({}),\n", join_path(path)))
                    }

                    value.push_str("  $env:PATH\n");
                    value.push_str(") -join [IO.PATH]::PathSeparator");
                    value
                }
                Export::Var(key, value) => {
                    if value.contains('/') {
                        format!("$env:{key} = {}", join_path(value))
                    } else {
                        format!(r#"$env:{key} = "{}""#, format(value))
                    }
                }
            }
        }
        _ => return None,
    };

    Some(result)
}

pub fn format_exports(shell: &Shell, comment: &str, exports: Vec<Export>) -> Option<String> {
    let mut lines = vec![format!("\n# {comment}")];

    for export in exports {
        match format_export(shell, export) {
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

    fn get_env_vars() -> Vec<Export> {
        vec![
            Export::Var("PROTO_HOME".into(), "$HOME/.proto".into()),
            Export::Path(vec!["$PROTO_HOME/shims".into(), "$PROTO_HOME/bin".into()]),
        ]
    }

    #[test]
    fn formats_bash_env_vars() {
        assert_eq!(
            format_exports(&Shell::Bash, "Bash", get_env_vars()).unwrap(),
            r#"
# Bash
export PROTO_HOME="$HOME/.proto"
export PATH="$PROTO_HOME/shims:$PROTO_HOME/bin:$PATH""#
        );
    }

    #[test]
    fn formats_elvish_env_vars() {
        assert_eq!(
            format_exports(&Shell::Elvish, "Elvish", get_env_vars()).unwrap(),
            r#"
# Elvish
set-env PROTO_HOME {~}/.proto
set paths [$E:PROTO_HOME/shims $E:PROTO_HOME/bin $@paths]"#
        );
    }

    #[test]
    fn formats_fish_env_vars() {
        assert_eq!(
            format_exports(&Shell::Fish, "Fish", get_env_vars()).unwrap(),
            r#"
# Fish
set -gx PROTO_HOME "$HOME/.proto"
set -gx PATH "$PROTO_HOME/shims:$PROTO_HOME/bin" $PATH"#
        );
    }

    #[test]
    fn formats_pwsh_env_vars() {
        assert_eq!(
            format_exports(&Shell::PowerShell, "PowerShell", get_env_vars()).unwrap(),
            r#"
# PowerShell
$env:PROTO_HOME = Join-Path $HOME ".proto"
$env:PATH = @(
  (Join-Path $env:PROTO_HOME "shims"),
  (Join-Path $env:PROTO_HOME "bin"),
  $env:PATH
) -join [IO.PATH]::PathSeparator"#
        );
    }
}
