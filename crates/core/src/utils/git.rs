use super::process::{ProtoProcessError, exec_command_piped, handle_exec};
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
    .args(["--depth", "1"]);

    // A shallow clone only fetches the default branch, so the
    // reference must be cloned directly to be checked out
    if let Some(reference) = &git.reference {
        cmd.args(["--branch", reference]);
    }

    cmd.arg(&git.url).arg(".").current_dir(cwd);
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

pub async fn clone_or_pull_repo(
    src: &GitSource,
    target_dir: &Path,
) -> Result<(), ProtoProcessError> {
    fs::create_dir_all(target_dir)?;

    let mut update_last_pull = false;
    let last_pull_path = target_dir.join(".last-pull");

    if target_dir.join(".git").exists() {
        let mut should_pull = true;

        if last_pull_path.exists()
            && let Some(last_timestamp) = fs::read_file(&last_pull_path)
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

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;
    use std::process::Command as StdCommand;
    use url::Url;

    fn run_git(args: &[&str], cwd: &Path) -> String {
        let output = StdCommand::new("git")
            .args([
                "-c",
                "user.name=proto",
                "-c",
                "user.email=proto@moonrepo.dev",
            ])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();

        assert!(output.status.success(), "{output:?}");

        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn clones_reference_that_is_not_the_default_branch() {
        let sandbox = create_empty_sandbox();
        let remote_dir = sandbox.path().join("remote");
        let target_dir = sandbox.path().join("target");

        fs::create_dir_all(&remote_dir).unwrap();
        run_git(&["init", "--initial-branch", "main"], &remote_dir);
        run_git(&["commit", "--allow-empty", "-m", "first"], &remote_dir);
        run_git(&["tag", "v1"], &remote_dir);
        run_git(&["branch", "feature"], &remote_dir);
        run_git(&["commit", "--allow-empty", "-m", "second"], &remote_dir);

        let first = run_git(&["rev-parse", "v1"], &remote_dir);
        // Local paths ignore `--depth`, so a file URL is required
        let url = Url::from_file_path(&remote_dir).unwrap().to_string();

        for reference in ["v1", "feature"] {
            let dir = target_dir.join(reference);

            clone_or_pull_repo(
                &GitSource {
                    url: url.clone(),
                    reference: Some(reference.into()),
                    submodules: false,
                },
                &dir,
            )
            .await
            .unwrap();

            assert_eq!(run_git(&["rev-parse", "HEAD"], &dir), first);
        }
    }
}
