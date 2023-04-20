use crate::describer::Describable;
use crate::errors::ProtoError;
use starbase_utils::fs::{self, FsError};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tar::Archive;
use tracing::debug;
use zip::ZipArchive;

#[async_trait::async_trait]
pub trait Installable<'tool>: Send + Sync + Describable<'tool> {
    /// Return a prefix that will be removed from all paths when
    /// unpacking an archive and copying the files.
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }

    /// Return an absolute file path to the directory containing the installed tool.
    /// This is typically ~/.proto/tools/<tool>/<version>.
    fn get_install_dir(&self) -> Result<PathBuf, ProtoError>;

    /// Run any installation steps after downloading and verifying the tool.
    /// This is typically unzipping an archive, and running any installers/binaries.
    async fn install(&self, install_dir: &Path, download_path: &Path) -> Result<bool, ProtoError> {
        if install_dir.exists() {
            debug!("Tool already installed, continuing");

            return Ok(false);
        }

        if !download_path.exists() {
            return Err(ProtoError::InstallMissingDownload(self.get_name()));
        }

        let prefix = self.get_archive_prefix()?;

        debug!(
            download_file = %download_path.display(),
            install_dir = %install_dir.display(),
            "Attempting to install tool",
        );

        if unpack(download_path, install_dir, prefix)? {
            // Unpacked archive
        } else {
            let install_path = install_dir.join(if cfg!(windows) {
                format!("{}.exe", self.get_bin_name())
            } else {
                self.get_bin_name().to_string()
            });

            // Not an archive, assume a binary and copy
            fs::rename(download_path, &install_path)?;
            fs::update_perms(install_path, None)?;
        }

        debug!("Successfully installed tool");

        Ok(true)
    }

    /// Uninstall the tool by deleting the install directory.
    async fn uninstall(&self, install_dir: &Path) -> Result<bool, ProtoError> {
        if !install_dir.exists() {
            debug!("Tool has not been installed, aborting");

            return Ok(false);
        }

        debug!(
            install_dir = %install_dir.display(),
            "Deleting install directory"
        );

        fs::remove_dir_all(install_dir)?;

        debug!("Successfully uninstalled tool");

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

    match ext {
        Some("zip") => unzip(input_file, output_dir, remove_prefix)?,
        Some("tgz" | "gz") => untar_gzip(input_file, output_dir, remove_prefix)?,
        Some("txz" | "xz") => untar_xzip(input_file, output_dir, remove_prefix)?,
        None => {
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

#[tracing::instrument(skip_all)]
pub fn untar<I: AsRef<Path>, O: AsRef<Path>, R: FnOnce(File) -> D, D: Read>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
    decoder: R,
) -> Result<(), ProtoError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();
    let handle_input_error = |error: io::Error| FsError::Read {
        path: input_file.to_path_buf(),
        error,
    };

    debug!(
        input_file = %input_file.display(),
        output_dir = %output_dir.display(),
        "Unpacking tar archive",
    );

    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    // Open .tar.gz file
    let tar_gz = fs::open_file(input_file)?;

    // Decompress to .tar
    let tar = decoder(tar_gz);

    // Unpack the archive into the output dir
    let mut archive = Archive::new(tar);

    for entry_result in archive.entries().map_err(handle_input_error)? {
        let mut entry = entry_result.map_err(handle_input_error)?;
        let mut path: PathBuf = entry.path().map_err(handle_input_error)?.into_owned();

        // Remove the prefix
        if let Some(prefix) = &remove_prefix {
            if path.starts_with(prefix) {
                path = path.strip_prefix(prefix).unwrap().to_owned();
            }
        }

        let output_path = output_dir.join(path);

        // Create parent dirs
        if let Some(parent_dir) = output_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        entry.unpack(&output_path).map_err(|error| FsError::Write {
            path: output_path.to_path_buf(),
            error,
        })?;
    }

    Ok(())
}

pub fn untar_gzip<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
) -> Result<(), ProtoError> {
    untar(input_file, output_dir, remove_prefix, |file| {
        flate2::read::GzDecoder::new(file)
    })
}

pub fn untar_xzip<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
) -> Result<(), ProtoError> {
    untar(input_file, output_dir, remove_prefix, |file| {
        xz2::read::XzDecoder::new(file)
    })
}

#[tracing::instrument(skip_all)]
pub fn unzip<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
) -> Result<(), ProtoError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();

    debug!(
        input_file = %input_file.display(),
        output_dir = %output_dir.display(),
        "Unzipping zip archive"
    );

    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    // Open .zip file
    let zip = fs::open_file(input_file)?;

    // Unpack the archive into the output dir
    let mut archive = ZipArchive::new(zip).map_err(ProtoError::Zip)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(ProtoError::Zip)?;

        let mut path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        // Remove the prefix
        if let Some(prefix) = &remove_prefix {
            if path.starts_with(prefix) {
                path = path.strip_prefix(prefix).unwrap().to_owned();
            }
        }

        let output_path = output_dir.join(&path);

        // Create parent dirs
        if let Some(parent_dir) = &output_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }

        // If a folder, create the dir
        if file.is_dir() {
            fs::create_dir_all(&output_path)?;
        }

        // If a file, copy it to the output dir
        if file.is_file() {
            let mut out = fs::create_file(&output_path)?;

            io::copy(&mut file, &mut out).map_err(|error| FsError::Write {
                path: output_path.to_path_buf(),
                error,
            })?;

            fs::update_perms(&output_path, file.unix_mode())?;
        }
    }

    Ok(())
}
