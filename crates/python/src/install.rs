use crate::PythonLanguage;
use proto_core::{async_trait, color, Describable, Installable, ProtoError, Resolvable};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

fn handle_error(e: std::io::Error) -> ProtoError {
    ProtoError::Message(format!(
        "Failed to run {}: {}",
        color::shell("rye"),
        color::muted_light(e.to_string())
    ))
}

async fn is_installed_in_rye(install_dir: &Path) -> Result<bool, ProtoError> {
    let output = Command::new("rye")
        .args(["toolchain", "list"])
        .output()
        .await
        .map_err(handle_error)?;

    let installed_list = String::from_utf8_lossy(&output.stdout);
    let install_target = install_dir
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    Ok(installed_list.contains(&install_target))
}

async fn run_rye_toolchain(command: &str, version: &str) -> Result<bool, ProtoError> {
    let status = Command::new("rye")
        .args(["toolchain", command, version])
        .spawn()
        .map_err(handle_error)?
        .wait()
        .await
        .map_err(handle_error)?;

    Ok(status.success())
}

#[async_trait]
impl Installable<'_> for PythonLanguage {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        // TODO ??
        let target = "3.11.3".to_owned();

        // ~/.rustup/toolchains/1.68.0-aarch64-apple-darwin
        Ok(self
            .rye_dir
            .join(format!("{}-{}", self.get_resolved_version(), target)))
    }

    async fn install(&self, install_dir: &Path, _download_path: &Path) -> Result<bool, ProtoError> {
        if is_installed_in_rye(install_dir).await? {
            debug!(
                tool = self.get_id(),
                "Toolchain already installed, continuing"
            );

            return Ok(false);
        }

        let success = run_rye_toolchain("fetch", self.get_resolved_version()).await?;

        debug!(tool = self.get_id(), "Successfully installed tool");

        Ok(success)
    }

    async fn uninstall(&self, install_dir: &Path) -> Result<bool, ProtoError> {
        if !is_installed_in_rye(install_dir).await? {
            debug!(
                tool = self.get_id(),
                "Tool has not been installed, aborting"
            );

            return Ok(false);
        }

        let success = run_rye_toolchain("remove", self.get_resolved_version()).await?;

        debug!(tool = self.get_id(), "Successfully uninstalled tool");

        Ok(success)
    }
}
