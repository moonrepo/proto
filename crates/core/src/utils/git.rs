use proto_pdk_api::GitSource;
use std::path::Path;
use tokio::process::Command;

pub fn clone(git: &GitSource, cwd: &Path) -> Command {
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

pub fn checkout(reference: &str, cwd: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("checkout").arg(reference).current_dir(cwd);
    cmd
}

pub fn pull(cwd: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(["pull", "--ff", "--prune"]).current_dir(cwd);
    cmd
}
