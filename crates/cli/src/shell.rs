use crate::error::ProtoCliError;
use iocraft::prelude::element;
use proto_core::reporter::ProtoConsole;
use starbase_console::ui::*;
use starbase_shell::{BoxedShell, ShellType};
use starbase_styles::color;
use starbase_utils::envx;
use starbase_utils::fs::{self, FsError};
use std::{
    env::consts,
    io::{self, BufRead},
    path::{Path, PathBuf},
};
use tracing::debug;

pub enum Export {
    Path(Vec<String>),
    Var(String, String),
}

// Fish sources every `*.fish` file in `conf.d` before `config.fish`, so proto
// can be set up in a file of its own. This is an option, not the default,
// because anything that `config.fish` then prepends to `PATH`, like Homebrew's
// `brew shellenv`, takes precedence over proto's shims.
fn get_fish_conf_path(shell: &BoxedShell, home_dir: &Path) -> PathBuf {
    shell
        .get_config_path(home_dir)
        .with_file_name("conf.d")
        .join("proto.fish")
}

fn get_profile_from_env() -> Option<PathBuf> {
    envx::path_var("PROTO_SHELL_PROFILE")
}

pub fn find_profiles(shell_type: ShellType, home_dir: &Path) -> Vec<PathBuf> {
    debug!("Finding profile files for {}", shell_type);

    if let Some(profile) = get_profile_from_env() {
        return vec![profile];
    }

    let shell = shell_type.build();
    let mut profiles = shell.get_profile_paths(home_dir);

    if shell_type == ShellType::Fish {
        profiles.push(get_fish_conf_path(&shell, home_dir));
    }

    profiles
}

/// Find the profile to set up when not prompting. The profile from
/// `PROTO_SHELL_PROFILE` is used as-is, even when it doesn't exist yet.
/// Otherwise a profile that has already been set up is preferred, so that
/// another profile isn't set up as well, then the first profile that exists,
/// and lastly the common profile for setting env vars.
pub fn find_setup_profile(shell_type: ShellType, home_dir: &Path, env_var: &str) -> PathBuf {
    if let Some(profile) = get_profile_from_env() {
        return profile;
    }

    let profiles = find_profiles(shell_type, home_dir);

    if let Some(profile) = profiles
        .iter()
        .find(|profile| is_profile_setup(profile, env_var))
    {
        return profile.to_path_buf();
    }

    if let Some(profile) = profiles.iter().find(|profile| profile.exists()) {
        return profile.to_path_buf();
    }

    shell_type.build().get_env_path(home_dir)
}

// Only used to inform the choice of a profile, so one that can't be read
// counts as not set up
fn is_profile_setup(profile: &Path, env_var: &str) -> bool {
    profile.exists() && has_profile_setup(profile, env_var).unwrap_or(false)
}

fn has_profile_setup(profile: &Path, env_var: &str) -> Result<bool, FsError> {
    let file = fs::open_file(profile)?;

    Ok(io::BufReader::new(file)
        .lines()
        .any(|line| line.is_ok_and(|l| l.contains(env_var))))
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
            Export::Path(paths) => shell.format_path_prepend(&paths),
            Export::Var(key, value) => shell.format_env_set(&key, &value),
        });
    }

    lines.join(newline)
}

pub fn update_profile(profile: &Path, contents: &str, env_var: &str) -> Result<(), FsError> {
    debug!("Updating profile {} with {}", color::path(profile), env_var);

    fs::append_file(profile, contents)?;

    Ok(())
}

pub fn update_profile_if_not_setup(
    profile: &Path,
    contents: &str,
    env_var: &str,
) -> Result<bool, FsError> {
    if !profile.exists() {
        update_profile(profile, contents, env_var)?;

        return Ok(true);
    }

    debug!(
        "Checking if profile {} has already been setup for {}",
        color::path(profile),
        env_var
    );

    // Already setup profile, so avoid writing
    if has_profile_setup(profile, env_var)? {
        debug!("Profile already setup");

        return Ok(false);
    }

    debug!("Not setup, continuing");

    update_profile(profile, contents, env_var)?;

    Ok(true)
}

pub async fn prompt_for_shell(console: &ProtoConsole) -> Result<ShellType, ProtoCliError> {
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
                on_index: &mut selected_index,
            )
        })
        .await?;

    Ok(options[selected_index])
}

pub async fn prompt_for_shell_profile(
    console: &ProtoConsole,
    shell_type: ShellType,
    home_dir: &Path,
    env_var: &str,
) -> Result<Option<PathBuf>, ProtoCliError> {
    let profiles = find_profiles(shell_type, home_dir);
    let fish_conf_path =
        (shell_type == ShellType::Fish).then(|| get_fish_conf_path(&shell_type.build(), home_dir));

    let mut options = profiles
        .iter()
        .map(|path| {
            let option = SelectOption::new(path.to_string_lossy());

            if is_profile_setup(path, env_var) {
                option.description("Already set up")
            } else if fish_conf_path.as_ref().is_some_and(|conf| conf == path) {
                option.description("Loaded before config.fish, so PATH changes there come first")
            } else {
                option
            }
        })
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
                on_index: Some(&mut selected_index),
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
                    on_value: &mut custom_path,
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
