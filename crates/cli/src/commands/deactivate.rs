use crate::app::StdoutOwner;
use crate::commands::activate::{
    ACTIVATED_ALIASES_KEY, ACTIVATED_ENV_KEY, ACTIVATED_PATH_KEY, ActivateOutput,
};
use crate::session::{ProtoSession, SessionResult};
use crate::workflows::{convert_paths_for_shell, join_paths_for_shell, remove_activated_paths};
use clap::Args;
use indexmap::IndexMap;
use starbase_shell::ShellType;
use starbase_utils::envx;
use std::env;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct DeactivateArgs {
    #[arg(help = "Shell to deactivate for")]
    shell: Option<ShellType>,

    #[arg(
        long,
        help = "Print the deactivate instructions in shell specific-syntax"
    )]
    pub export: bool,
}

#[instrument(skip(session))]
pub async fn deactivate(session: ProtoSession, args: DeactivateArgs) -> SessionResult {
    // Detect the shell that we need to deactivate for
    let shell_type = match args.shell {
        Some(value) => value,
        None => ShellType::try_detect()?,
    };

    // Everything that must be reversed is tracked by the activation itself,
    // so configuration and tools are never loaded.
    let env_keys = list_tracked_keys(ACTIVATED_ENV_KEY);
    let alias_names = list_tracked_keys(ACTIVATED_ALIASES_KEY);
    let tracking_keys = list_set_tracking_keys();
    let paths = envx::paths();
    let next_paths = remove_activated_paths(&session.env.store.dir, paths.clone());
    let next_paths = (next_paths.len() != paths.len()).then_some(next_paths);

    // Shell code is this command's default protocol; see `App::stdout_owner`
    match session.cli.stdout_owner() {
        StdoutOwner::Reporter => {
            let mut env = IndexMap::default();

            for key in env_keys.into_iter().chain(tracking_keys) {
                env.insert(key, None);
            }

            session.console.write_json_for_format(ActivateOutput {
                path: match next_paths {
                    Some(paths) => join_paths_for_shell(paths.iter(), &shell_type)?
                        .into_string()
                        .ok(),
                    None => None,
                },
                env,
            })?;
        }
        StdoutOwner::ShellCode => {
            print_deactivation_exports(
                &session,
                &shell_type,
                env_keys,
                alias_names,
                tracking_keys,
                next_paths,
            )?;
        }
        StdoutOwner::CompletionCode | StdoutOwner::McpStdio => {
            unreachable!("deactivate resolved to an unrelated stdout owner")
        }
    };

    Ok(None)
}

fn print_deactivation_exports(
    session: &ProtoSession,
    shell_type: &ShellType,
    env_keys: Vec<String>,
    alias_names: Vec<String>,
    tracking_keys: Vec<String>,
    next_paths: Option<Vec<PathBuf>>,
) -> miette::Result<()> {
    let shell = shell_type.build();
    let mut output = vec![];

    // Remove the variables and aliases that were set
    for key in env_keys {
        output.push(shell.format_env_unset(&key));
    }

    for name in alias_names {
        output.push(shell.format_alias_unset(&name));
    }

    // Reset `PATH` to what it was before activating
    if let Some(next_paths) = next_paths {
        let paths = convert_paths_for_shell(next_paths.iter(), shell_type)
            .into_iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        output.push(shell.format_path_set(&paths));
    }

    // And lastly, stop tracking the activation itself
    for key in tracking_keys {
        output.push(shell.format_env_unset(&key));
    }

    if !output.is_empty() {
        session.console.out.write_line(output.join("\n"))?;
    }

    Ok(())
}

/// Return the keys/names that a tracking variable recorded, if activated.
fn list_tracked_keys(tracking_key: &str) -> Vec<String> {
    env::var(tracking_key)
        .map(|value| split_tracked_keys(&value))
        .unwrap_or_default()
}

/// Return the tracking variables that are currently set.
fn list_set_tracking_keys() -> Vec<String> {
    [ACTIVATED_ENV_KEY, ACTIVATED_ALIASES_KEY, ACTIVATED_PATH_KEY]
        .into_iter()
        .filter(|key| env::var(key).is_ok())
        .map(|key| key.to_owned())
        .collect()
}

fn split_tracked_keys(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter(|key| !key.is_empty())
        .map(|key| key.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_tracked_keys() {
        assert_eq!(split_tracked_keys("KEY1,KEY2"), vec!["KEY1", "KEY2"]);
    }

    #[test]
    fn ignores_empty_tracked_keys() {
        assert!(split_tracked_keys("").is_empty());
        assert_eq!(split_tracked_keys("KEY1,,KEY2,"), vec!["KEY1", "KEY2"]);
    }
}
