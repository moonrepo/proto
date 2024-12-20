use crate::session::ProtoConsole;
use iocraft::prelude::element;
use starbase_console::ui::*;
use starbase_shell::{BoxedShell, ShellType};
use starbase_styles::color;
use starbase_utils::fs;
use std::{
    env::{self, consts},
    io::{self, BufRead},
    path::{Path, PathBuf},
};
use tracing::debug;

pub enum Export {
    Path(Vec<String>),
    Var(String, String),
}

pub fn find_profiles(shell: &BoxedShell, home_dir: &Path) -> miette::Result<Vec<PathBuf>> {
    debug!("Finding profile files for {}", shell);

    if let Ok(profile_env) = env::var("PROTO_SHELL_PROFILE") {
        return Ok(vec![PathBuf::from(profile_env)]);
    }

    Ok(shell.get_profile_paths(home_dir))
}

pub fn find_first_profile(shell: &BoxedShell, home_dir: &Path) -> miette::Result<PathBuf> {
    for profile in find_profiles(shell, home_dir)? {
        if profile.exists() {
            return Ok(profile);
        }
    }

    // Otherwise return the common profile for setting env vars
    Ok(shell.get_env_path(home_dir))
}

pub fn format_exports(shell: &BoxedShell, comment: &str, exports: Vec<Export>) -> String {
    let newline = if consts::OS == "windows" {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines = vec![format!("{newline}# {comment}")];

    for export in exports {
        lines.push(match export {
            Export::Path(paths) => shell.format_path_set(&paths),
            Export::Var(key, value) => shell.format_env_set(&key, &value),
        });
    }

    lines.join(newline)
}

pub fn update_profile(profile: &Path, contents: &str, env_var: &str) -> miette::Result<()> {
    debug!("Updating profile {} with {}", color::path(profile), env_var);

    fs::append_file(profile, contents)?;

    Ok(())
}

pub fn update_profile_if_not_setup(
    profile: &Path,
    contents: &str,
    env_var: &str,
) -> miette::Result<bool> {
    if !profile.exists() {
        update_profile(profile, contents, env_var)?;

        return Ok(true);
    }

    debug!(
        "Checking if profile {} has already been setup for {}",
        color::path(profile),
        env_var
    );

    let file = fs::open_file(profile)?;
    let has_setup = io::BufReader::new(file)
        .lines()
        .any(|line| line.is_ok_and(|l| l.contains(env_var)));

    // Already setup profile, so avoid writing
    if has_setup {
        debug!("Profile already setup");

        return Ok(false);
    }

    debug!("Not setup, continuing");

    update_profile(profile, contents, env_var)?;

    Ok(true)
}

pub async fn prompt_for_shell(console: &ProtoConsole) -> miette::Result<ShellType> {
    let options = ShellType::os_variants();
    let mut selected_index = 0;

    console
        .render_interactive(element! {
            Select(
                label: "Which shell to use and configure?",
                options: options
                    .iter()
                    .map(|item| SelectOption::new(item.to_string()))
                    .collect::<Vec<_>>(),
                value: Some(&mut selected_index),
            )
        })
        .await?;

    Ok(options[selected_index])
}

pub async fn prompt_for_shell_profile(
    console: &ProtoConsole,
    shell: &BoxedShell,
    home_dir: &Path,
) -> miette::Result<Option<PathBuf>> {
    let profiles = find_profiles(shell, home_dir)?;

    let mut options = profiles
        .iter()
        .map(|path| SelectOption::new(path.to_string_lossy()))
        .collect::<Vec<_>>();
    options.push(SelectOption::new("Other"));
    options.push(SelectOption::new("None"));

    let mut selected_index = 0;
    let other_index = profiles.len();
    let none_index = other_index + 1;

    console
        .render_interactive(element! {
            Select(
                label: "Which profile or config file to update?",
                options,
                value: Some(&mut selected_index),
            )
        })
        .await?;

    let selected_profile = if selected_index == none_index {
        None
    } else if selected_index == other_index {
        let mut custom_path = String::new();

        console
            .render_interactive(element! {
                Input(
                    label: "Custom profile path?",
                    value: Some(&mut custom_path),
                    validate: |new_value: String| {
                        if new_value.is_empty() || !PathBuf::from(new_value).is_absolute() {
                            Some("An absolute path is required".into())
                        } else {
                            None
                        }
                    }
                )
            })
            .await?;

        Some(PathBuf::from(custom_path))
    } else {
        Some(profiles[selected_index].clone())
    };

    Ok(selected_profile)
}
