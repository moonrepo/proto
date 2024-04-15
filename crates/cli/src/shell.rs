use clap_complete::Shell;
use dialoguer::{Input, Select};
use dirs::{config_dir, document_dir, home_dir};
use miette::IntoDiagnostic;
use proto_core::ENV_VAR;
use starbase_styles::color;
use starbase_utils::fs;
use std::{
    env,
    io::{self, BufRead},
    path::{Path, PathBuf},
};
use tracing::debug;

use crate::helpers::create_theme;

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

            if let Some(dir) = config_dir() {
                profiles.push(dir.join("elvish/rc.elv"));
            }

            if cfg!(unix) {
                profiles.push(home_dir.join(".config/elvish/rc.elv"));
            }
        }
        Shell::Fish => {
            profiles.push(home_dir.join(".config/fish/config.fish"));
        }
        Shell::PowerShell => {
            if cfg!(windows) {
                let docs_dir = document_dir().unwrap_or(home_dir.join("Documents"));

                profiles.extend([
                    docs_dir.join("PowerShell\\Microsoft.PowerShell_profile.ps1"),
                    docs_dir.join("PowerShell\\Profile.ps1"),
                ]);
            } else {
                profiles.extend([
                    home_dir.join(".config/powershell/Microsoft.PowerShell_profile.ps1"),
                    home_dir.join(".config/powershell/profile.ps1"),
                ]);
            }
        }
        Shell::Zsh => {
            let zdot_dir = env::var("ZDOTDIR").map(PathBuf::from).unwrap_or(home_dir);

            profiles.extend([
                zdot_dir.join(".zshenv"),
                zdot_dir.join(".zprofile"),
                zdot_dir.join(".zshrc"),
            ]);
        }
        _ => {}
    };

    Ok(profiles)
}

pub fn format_export(shell: &Shell, var: Export, newline: &str) -> Option<String> {
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
                    .split('/')
                    .map(|part| {
                        if part.starts_with('$') {
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
                    let mut value = format!("$env:PATH = @({newline}");

                    for path in paths {
                        value.push_str(&format!("  ({}),{newline}", join_path(path)))
                    }

                    value.push_str("  $env:PATH");
                    value.push_str(newline);
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
    let newline = if matches!(shell, Shell::PowerShell) {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines = vec![format!("{newline}# {comment}")];

    for export in exports {
        match format_export(shell, export, newline) {
            Some(var) => lines.push(var),
            None => return None,
        };
    }

    Some(lines.join(newline))
}

pub fn write_profile(profile: &Path, contents: &str, env_var: &str) -> miette::Result<PathBuf> {
    fs::append_file(profile, contents)?;

    debug!("Setup profile {} with {}", color::path(profile), env_var,);

    Ok(profile.to_path_buf())
}

pub fn write_profile_if_not_setup(
    shell: &Shell,
    contents: &str,
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

    Ok(Some(write_profile(last_profile, contents, env_var)?))
}

pub fn prompt_for_shell_profile(shell: &Shell, cwd: &Path) -> miette::Result<Option<PathBuf>> {
    let theme = create_theme();

    let mut profiles = find_profiles(shell)?;
    profiles.reverse();

    let mut items = profiles.iter().map(color::path).collect::<Vec<_>>();
    items.push("Other".to_owned());
    items.push("None".to_owned());

    let default_index = 0;
    let other_index = profiles.len();
    let none_index = other_index + 1;

    let selected_index = Select::with_theme(&theme)
        .with_prompt("Which profile to update?")
        .items(&items)
        .default(default_index)
        .interact_opt()
        .into_diagnostic()?
        .unwrap_or(default_index);

    let selected_profile = if selected_index == none_index {
        None
    } else if selected_index == other_index {
        let custom_path = PathBuf::from(
            Input::<String>::with_theme(&theme)
                .with_prompt("Custom profile path?")
                .interact_text()
                .into_diagnostic()?,
        );

        Some(if custom_path.is_absolute() {
            custom_path
        } else {
            cwd.join(custom_path)
        })
    } else {
        Some(profiles[selected_index].clone())
    };

    Ok(selected_profile)
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
            format_exports(&Shell::PowerShell, "PowerShell", get_env_vars())
                .unwrap()
                .replace("\r\n", "\n"),
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
