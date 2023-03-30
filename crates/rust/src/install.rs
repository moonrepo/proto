use crate::RustLanguage;
use log::debug;
use proto_core::{async_trait, color, Describable, Installable, ProtoError, Resolvable};
use std::env::consts;
use std::path::{Path, PathBuf};
use tokio::process::Command;

fn is_musl() -> bool {
    let Ok(output) = std::process::Command::new("ldd").arg("--version").output() else {
        return false;
    };

    String::from_utf8_lossy(&output.stdout).contains("musl")
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
        let handle_error = |e: std::io::Error| {
            ProtoError::Message(format!(
                "Failed to run {}: {}",
                color::shell("rustup"),
                color::muted_light(e.to_string())
            ))
        };

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

        if installed_list.contains(&install_target) {
            debug!(target: self.get_log_target(), "Toolchain already installed, continuing");

            return Ok(false);
        }

        let mut cmd = Command::new("rustup");
        cmd.args(["toolchain", "install", self.get_resolved_version()]);

        let status = cmd
            .spawn()
            .map_err(handle_error)?
            .wait()
            .await
            .map_err(handle_error)?;

        Ok(status.success())
    }
}
