use crate::WasmPlugin;
use proto_core::{async_trait, unpack, Describable, Installable, ProtoError, Resolvable};
use proto_pdk::{InstallParams, UnpackArchiveInput};
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[async_trait]
impl Installable<'_> for WasmPlugin {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        let params: InstallParams =
            self.cache_func_with("register_install_params", self.get_env_input())?;

        Ok(params.archive_prefix)
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.base_dir.join(self.get_resolved_version()))
    }

    async fn install(&self, install_dir: &Path, download_path: &Path) -> Result<bool, ProtoError> {
        if install_dir.exists() {
            debug!(tool = self.get_id(), "Tool already installed, continuing");

            return Ok(false);
        }

        if !download_path.exists() {
            return Err(ProtoError::InstallMissingDownload(self.get_name()));
        }

        let prefix = self.get_archive_prefix()?;

        debug!(
            tool = self.get_id(),
            download_file = ?download_path,
            install_dir = ?install_dir,
            "Attempting to install tool",
        );

        if self.has_func("unpack_archive") {
            self.call_func_with(
                "unpack_archive",
                UnpackArchiveInput {
                    download_path: self.to_wasi_virtual_path(download_path),
                    install_dir: self.to_wasi_virtual_path(install_dir),
                },
            )?;
        } else if self.should_unpack() && unpack(download_path, install_dir, prefix)? {
            // Unpacked archive
        } else {
            let install_path = install_dir.join(if cfg!(windows) {
                format!("{}.exe", self.get_id())
            } else {
                self.get_id().to_string()
            });

            // Not an archive, assume a binary and copy
            fs::rename(download_path, &install_path)?;
            fs::update_perms(install_path, None)?;
        }

        debug!(tool = self.get_id(), "Successfully installed tool");

        Ok(true)
    }
}
