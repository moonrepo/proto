use super::process::{exec_command_piped, handle_exec};
use crate::helpers::now;
use proto_pdk_api::GitSource;
use starbase_utils::fs;
use std::path::Path;
use std::time::SystemTime;
use tokio::process::Command;

pub fn new_clone(git: &GitSource, cwd: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(if git.submodules {
        vec!["clone", "--recurse-submodules"]
    } else {
        vec!["clone"]
    })
    .args(["--depth", "1"])
    .arg(&git.url)
    .arg(".")
    .current_dir(cwd);
    cmd
}

pub fn new_checkout(reference: &str, cwd: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("checkout").arg(reference).current_dir(cwd);
    cmd
}

pub fn new_pull(cwd: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["pull", "--ff", "--prune"]).current_dir(cwd);
    cmd
}

pub async fn clone_or_pull_repo(src: &GitSource, target_dir: &Path) -> miette::Result<()> {
    fs::create_dir_all(target_dir)?;

    let mut update_last_pull = false;
    let last_pull_path = target_dir.join(".last-pull");

    if target_dir.join(".git").exists() {
        let mut should_pull = true;

        if last_pull_path.exists() {
            if let Some(last_timestamp) = fs::read_file(&last_pull_path)
                .ok()
                .and_then(|value| value.parse::<u128>().ok())
            {
                let now_millis = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();

                // Every 7 days
                if (now_millis - last_timestamp) < (7 * 24 * 60 * 60 * 1000) {
                    should_pull = false;
                }
            }
        }

        if should_pull {
            update_last_pull = true;
            handle_exec(exec_command_piped(&mut new_pull(target_dir)).await?)?;
        }
    } else {
        handle_exec(exec_command_piped(&mut new_clone(src, target_dir)).await?)?;

        if let Some(reference) = &src.reference {
            handle_exec(exec_command_piped(&mut new_checkout(reference, target_dir)).await?)?;
        }

        update_last_pull = true;
    }

    if update_last_pull {
        fs::write_file(last_pull_path, now().to_string())?;
    }

    Ok(())
}
