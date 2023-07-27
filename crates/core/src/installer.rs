use crate::describer::Describable;
use crate::errors::ProtoError;
use starbase_archive::tar::TarUnpacker;
use starbase_archive::zip::ZipUnpacker;
use starbase_archive::Archiver;
use starbase_utils::fs::{self};
use std::path::{Path, PathBuf};
use tracing::debug;

#[async_trait::async_trait]
pub trait Installable<'tool>: Send + Sync + Describable<'tool> {
    /// Return a prefix that will be removed from all paths when
    /// unpacking an archive and copying the files.
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }

    /// Return an absolute file path to the directory containing the installed tool.
    /// This is typically `~/.proto/tools/<tool>/<version>`.
    fn get_install_dir(&self) -> Result<PathBuf, ProtoError>;

    /// Run any installation steps after downloading and verifying the tool.
    /// This is typically unzipping an archive, and running any installers/binaries.
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

        if self.should_unpack() && unpack(download_path, install_dir, prefix)? {
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

    /// Whether or not the downloaded file should be unpacked before installing.
    fn should_unpack(&self) -> bool {
        true
    }

    /// Uninstall the tool by deleting the install directory.
    async fn uninstall(&self, install_dir: &Path) -> Result<bool, ProtoError> {
        if !install_dir.exists() {
            debug!(
                tool = self.get_id(),
                "Tool has not been installed, aborting"
            );

            return Ok(false);
        }

        debug!(
            tool = self.get_id(),
            install_dir = ?install_dir,
            "Deleting install directory"
        );

        fs::remove_dir_all(install_dir)?;

        debug!(tool = self.get_id(), "Successfully uninstalled tool");

        Ok(true)
    }
}

pub fn unpack<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
) -> Result<bool, ProtoError> {
    let input_file = input_file.as_ref();
    let ext = input_file.extension().map(|e| e.to_str().unwrap());
    let mut archiver = Archiver::new(output_dir.as_ref(), input_file);

    if let Some(prefix) = remove_prefix.as_ref() {
        archiver.set_prefix(prefix);
    }

    match ext {
        Some("zip") => {
            archiver
                .unpack(ZipUnpacker::new)
                .map_err(|e| ProtoError::Message(e.to_string()))?;
        }
        Some("tar") => {
            archiver
                .unpack(TarUnpacker::new)
                .map_err(|e| ProtoError::Message(e.to_string()))?;
        }
        Some("tgz" | "gz") => {
            archiver
                .unpack(TarUnpacker::new_gz)
                .map_err(|e| ProtoError::Message(e.to_string()))?;
        }
        Some("txz" | "xz") => {
            archiver
                .unpack(TarUnpacker::new_xz)
                .map_err(|e| ProtoError::Message(e.to_string()))?;
        }
        Some("exe") | None => {
            return Ok(false);
        }
        _ => {
            return Err(ProtoError::UnsupportedArchiveFormat(
                input_file.to_path_buf(),
                ext.unwrap_or_default().to_string(),
            ))
        }
    };

    Ok(true)
}
