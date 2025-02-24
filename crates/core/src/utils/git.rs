use super::process::{exec_command_piped, handle_exec};
use proto_pdk_api::GitSource;
use starbase_utils::fs;
use std::path::Path;
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

    if target_dir.join(".git").exists() {
        handle_exec(exec_command_piped(&mut new_pull(target_dir)).await?)?;
    } else {
        handle_exec(exec_command_piped(&mut new_clone(src, target_dir)).await?)?;

        if let Some(reference) = &src.reference {
            handle_exec(exec_command_piped(&mut new_checkout(reference, target_dir)).await?)?;
        }
    }

    Ok(())
}
