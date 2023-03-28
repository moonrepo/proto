use crate::errors::ProtoError;
use crate::{color, Describable};
use log::{debug, trace};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use tar::Archive;
use zip::result::ZipError;
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
            debug!(target: self.get_log_target(), "Tool already installed, continuing");

            return Ok(false);
        }

        if !download_path.exists() {
            return Err(ProtoError::InstallMissingDownload(self.get_name()));
        }

        let prefix = self.get_archive_prefix()?;

        debug!(
            target: self.get_log_target(),
            "Attempting to install {} to {}",
            color::path(download_path),
            color::path(install_dir),
        );

        unpack(download_path, install_dir, prefix)?;

        debug!(target: self.get_log_target(), "Successfully installed tool");

        Ok(true)
    }
}

pub fn unpack<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
) -> Result<(), ProtoError> {
    let input_file = input_file.as_ref();
    let ext = input_file.extension().unwrap_or_default().to_string_lossy();

    match ext.as_ref() {
        "zip" => unzip(input_file, output_dir, remove_prefix),
        "gz" => untar_gzip(input_file, output_dir, remove_prefix),
        "xz" => untar_xzip(input_file, output_dir, remove_prefix),
        _ => Err(ProtoError::UnsupportedArchiveFormat(
            input_file.to_path_buf(),
            ext.to_string(),
        )),
    }
}

pub fn untar<I: AsRef<Path>, O: AsRef<Path>, R: FnOnce(File) -> D, D: Read>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
    decoder: R,
) -> Result<(), ProtoError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();
    let handle_input_error = |e: io::Error| ProtoError::Fs(input_file.to_path_buf(), e.to_string());
    let handle_output_error =
        |e: io::Error| ProtoError::Fs(output_dir.to_path_buf(), e.to_string());

    trace!(
        target: "proto:installer",
        "Unpacking tar archive {} to {}",
        color::path(input_file),
        color::path(output_dir),
    );

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(handle_output_error)?;
    }

    // Open .tar.gz file
    let tar_gz = File::open(input_file).map_err(handle_input_error)?;

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
            fs::create_dir_all(parent_dir)
                .map_err(|e| ProtoError::Fs(parent_dir.to_path_buf(), e.to_string()))?;
        }

        entry
            .unpack(&output_path)
            .map_err(|e| ProtoError::Fs(output_path.to_path_buf(), e.to_string()))?;
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

pub fn unzip<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<String>,
) -> Result<(), ProtoError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();
    let handle_input_error = |e: io::Error| ProtoError::Fs(input_file.to_path_buf(), e.to_string());
    let handle_output_error =
        |e: io::Error| ProtoError::Fs(output_dir.to_path_buf(), e.to_string());
    let handle_zip_error = |e: ZipError| ProtoError::Zip(e.to_string());

    trace!(
        target: "proto:installer",
        "Unzipping zip archive {} to {}",
        color::path(input_file),
        color::path(output_dir),
    );

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(handle_output_error)?;
    }

    // Open .zip file
    let zip = File::open(input_file).map_err(handle_input_error)?;

    // Unpack the archive into the output dir
    let mut archive = ZipArchive::new(zip).map_err(handle_zip_error)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(handle_zip_error)?;

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
        let handle_error = |e: io::Error| ProtoError::Fs(output_path.to_path_buf(), e.to_string());

        // Create parent dirs
        if let Some(parent_dir) = &output_path.parent() {
            fs::create_dir_all(parent_dir)
                .map_err(|e| ProtoError::Fs(parent_dir.to_path_buf(), e.to_string()))?;
        }

        // If a folder, create the dir
        if file.is_dir() {
            fs::create_dir_all(&output_path).map_err(handle_error)?;
        }

        // If a file, copy it to the output dir
        if file.is_file() {
            let mut out = File::create(&output_path).map_err(handle_error)?;

            io::copy(&mut file, &mut out).map_err(handle_error)?;

            // Update permissions when on a nix machine
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&output_path, fs::Permissions::from_mode(mode))
                        .map_err(handle_error)?;
                }
            }
        }
    }

    Ok(())
}
