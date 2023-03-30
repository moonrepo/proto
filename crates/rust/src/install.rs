use crate::RustLanguage;
use proto_core::{async_trait, color, Installable, ProtoError, Resolvable};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[async_trait]
impl Installable<'_> for RustLanguage {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.base_dir.join(self.get_resolved_version()))
    }

    async fn install(
        &self,
        _install_dir: &Path,
        _download_path: &Path,
    ) -> Result<bool, ProtoError> {
        let mut cmd = Command::new("rustup");
        cmd.args(["toolchain", "install", self.get_resolved_version()]);

        let handle_error = |e: std::io::Error| {
            ProtoError::Message(format!(
                "Failed to run {}: {}",
                color::shell("rustup"),
                color::muted_light(e.to_string())
            ))
        };

        let status = cmd
            .spawn()
            .map_err(handle_error)?
            .wait()
            .await
            .map_err(handle_error)?;

        Ok(status.success())
    }
}
