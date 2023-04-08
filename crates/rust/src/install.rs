use crate::RustLanguage;
use proto_core::{async_trait, color, Installable, ProtoError, Resolvable};
use std::env::consts;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::debug;

fn handle_error(e: std::io::Error) -> ProtoError {
    ProtoError::Message(format!(
        "Failed to run {}: {}",
        color::shell("rustup"),
        color::muted_light(e.to_string())
    ))
}

fn is_musl() -> bool {
    let Ok(output) = std::process::Command::new("ldd").arg("--version").output() else {
        return false;
    };

    String::from_utf8_lossy(&output.stdout).contains("musl")
}

async fn is_installed_in_rustup(install_dir: &Path) -> Result<bool, ProtoError> {
    let output = Command::new("rustup")
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

async fn run_rustup_toolchain(command: &str, version: &str) -> Result<bool, ProtoError> {
    let status = Command::new("rustup")
        .args(["toolchain", command, version])
        .spawn()
        .map_err(handle_error)?
        .wait()
        .await
        .map_err(handle_error)?;

    Ok(status.success())
}

#[async_trait]
impl Installable<'_> for RustLanguage {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        let target = match consts::OS {
            "linux" => format!(
                "{}-unknown-linux-{}",
                consts::ARCH,
                if is_musl() { "musl" } else { "gnu" }
            ),
            "macos" => format!("{}-apple-darwin", consts::ARCH),
            "windows" => format!("{}-pc-windows-msvc", consts::ARCH),
            other => return Err(ProtoError::UnsupportedPlatform("Rust".into(), other.into())),
        };

        // ~/.rustup/toolchains/1.68.0-aarch64-apple-darwin
        Ok(self
            .base_dir
            .join(format!("{}-{}", self.get_resolved_version(), target)))
    }

    async fn install(&self, install_dir: &Path, _download_path: &Path) -> Result<bool, ProtoError> {
        if is_installed_in_rustup(install_dir).await? {
            debug!("Toolchain already installed, continuing");

            return Ok(false);
        }

        let success = run_rustup_toolchain("install", self.get_resolved_version()).await?;

        debug!("Successfully installed tool");

        Ok(success)
    }

    async fn uninstall(&self, install_dir: &Path) -> Result<bool, ProtoError> {
        if !is_installed_in_rustup(install_dir).await? {
            debug!("Tool has not been installed, aborting");

            return Ok(false);
        }

        let success = run_rustup_toolchain("uninstall", self.get_resolved_version()).await?;

        debug!("Successfully uninstalled tool");

        Ok(success)
    }
}
