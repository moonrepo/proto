use crate::describer::Describable;
use crate::errors::ProtoError;
use crate::helpers::is_offline;
use crate::resolver::Resolvable;
use log::{debug, trace};
use starbase_styles::color;
use starbase_utils::fs::{self, FsError};
use std::io;
use std::path::{Path, PathBuf};

#[async_trait::async_trait]
pub trait Downloadable<'tool>: Send + Sync + Describable<'tool> + Resolvable<'tool> {
    /// Return an absolute file path to the downloaded file.
    /// This may not exist, as the path is composed ahead of time.
    /// This is typically ~/.proto/temp/<file>.
    fn get_download_path(&self) -> Result<PathBuf, ProtoError>;

    /// Return a URL to download the tool's archive from a registry.
    fn get_download_url(&self) -> Result<String, ProtoError>;

    /// Download the tool (as an archive) from its distribution registry
    /// into the ~/.proto/temp folder and return an absolute file path.
    /// A custom URL that points to the downloadable archive can be
    /// provided as the 2nd argument.
    async fn download(&self, to_file: &Path, from_url: Option<&str>) -> Result<bool, ProtoError> {
        if to_file.exists() {
            debug!(target: self.get_log_target(), "Tool already downloaded, continuing");

            return Ok(false);
        }

        let from_url = match from_url {
            Some(url) => url.to_owned(),
            None => self.get_download_url()?,
        };

        debug!(
            target: self.get_log_target(),
            "Attempting to download tool from {}",
            color::url(&from_url),
        );

        download_from_url(&from_url, &to_file).await?;

        debug!(target: self.get_log_target(), "Successfully downloaded tool");

        Ok(true)
    }
}

pub async fn download_from_url<U, F>(url: U, dest_file: F) -> Result<(), ProtoError>
where
    U: AsRef<str>,
    F: AsRef<Path>,
{
    if is_offline() {
        return Err(ProtoError::InternetConnectionRequired);
    }

    let url = url.as_ref();
    let dest_file = dest_file.as_ref();
    let handle_io_error = |error: io::Error| FsError::Create {
        path: dest_file.to_path_buf(),
        error,
    };
    let handle_http_error = |error: reqwest::Error| ProtoError::Http {
        url: url.to_owned(),
        error,
    };

    trace!(
        target: "proto:downloader",
        "Downloading {} from {}",
        color::path(dest_file),
        color::url(url)
    );

    // Ensure parent directories exist
    if let Some(parent) = dest_file.parent() {
        fs::create_dir_all(parent)?;
    }

    // Fetch the file from the HTTP source
    let response = reqwest::get(url).await.map_err(handle_http_error)?;
    let status = response.status();

    if !status.is_success() {
        return Err(ProtoError::DownloadFailed(
            url.to_owned(),
            status.to_string(),
        ));
    }

    // Write the bytes to our local file
    let mut contents = io::Cursor::new(response.bytes().await.map_err(handle_http_error)?);
    let mut file = fs::create_file(dest_file)?;

    io::copy(&mut contents, &mut file).map_err(handle_io_error)?;

    Ok(())
}
