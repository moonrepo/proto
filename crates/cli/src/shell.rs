use crate::helpers::create_theme;
use dialoguer::{Input, Select};
use miette::IntoDiagnostic;
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

    Ok(shell.get_profile_paths(&home_dir))
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
            Export::Path(paths) => shell.format_path_export(&paths),
            Export::Var(key, value) => shell.format_env_export(&key, &value),
        });
    }

    lines.join(newline)
}

pub fn write_profile(profile: &Path, contents: &str, env_var: &str) -> miette::Result<PathBuf> {
    fs::append_file(profile, contents)?;

    debug!("Setup profile {} with {}", color::path(profile), env_var);

    Ok(profile.to_path_buf())
}

pub fn write_profile_if_not_setup(
    shell: &BoxedShell,
    contents: &str,
    env_var: &str,
    home_dir: &Path,
) -> miette::Result<Option<PathBuf>> {
    let profiles = find_profiles(shell, home_dir)?;

    for profile in profiles {
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

    // If no profile found, use the env-specific profile for the shell
    let last_profile = shell.get_env_path(home_dir);

    debug!(
        "Found no configured profile, updating {}",
        color::path(last_profile),
    );

    Ok(Some(write_profile(&last_profile, contents, env_var)?))
}

pub fn prompt_for_shell() -> miette::Result<ShellType> {
    let theme = create_theme();
    let items = ShellType::os_variants();

    let default_index = 0;
    let selected_index = Select::with_theme(&theme)
        .with_prompt("Which shell to use?")
        .items(&items)
        .default(default_index)
        .interact_opt()
        .into_diagnostic()?
        .unwrap_or(default_index);

    Ok(items[selected_index])
}

pub fn prompt_for_shell_profile(
    shell: &BoxedShell,
    working_dir: &Path,
    home_dir: &Path,
) -> miette::Result<Option<PathBuf>> {
    let theme = create_theme();

    let mut profiles = find_profiles(shell, home_dir)?;
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
            working_dir.join(custom_path)
        })
    } else {
        Some(profiles[selected_index].clone())
    };

    Ok(selected_profile)
}
