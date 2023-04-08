use crate::downloader::{download_from_url, Downloadable};
use crate::errors::ProtoError;
use sha2::{Digest, Sha256};
use starbase_styles::color;
use starbase_utils::fs::{self, FsError};
use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

#[async_trait::async_trait]
pub trait Verifiable<'tool>: Send + Sync + Downloadable<'tool> {
    /// Return an absolute file path to the checksum file.
    /// This may not exist, as the path is composed ahead of time.
    /// This is typically ~/.prove/temp/<file>.
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError>;

    /// Return a URL to download the tool's checksum manifest from a registry.
    fn get_checksum_url(&self) -> Result<Option<String>, ProtoError>;

    /// If applicable, download all files necessary for verifying checksums.
    async fn download_checksum(
        &self,
        to_file: &Path,
        from_url: Option<&str>,
    ) -> Result<bool, ProtoError> {
        if to_file.exists() {
            debug!("Checksum already downloaded, continuing");

            return Ok(false);
        }

        let from_url = match from_url {
            Some(url) => Some(url.to_owned()),
            None => self.get_checksum_url()?,
        };

        // Not all tools requires a checksum!
        let Some(from_url) = from_url else {
            return Ok(true);
        };

        debug!(
            "Attempting to download checksum from {}",
            color::url(&from_url),
        );

        download_from_url(&from_url, &to_file).await?;

        debug!("Successfully downloaded checksum");

        Ok(true)
    }

    /// Verify the downloaded file using the checksum strategy for the tool.
    /// Common strategies are SHA256 and MD5.
    async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
    ) -> Result<bool, ProtoError>;
}

pub fn get_sha256_hash_of_file<P: AsRef<Path>>(path: P) -> Result<String, ProtoError> {
    let path = path.as_ref();

    trace!("Calculating SHA256 checksum for file {}", color::path(path));

    let mut file = fs::open_file(path)?;
    let mut sha = Sha256::new();

    io::copy(&mut file, &mut sha).map_err(|error| FsError::Read {
        path: path.to_path_buf(),
        error,
    })?;

    let hash = format!("{:x}", sha.finalize());

    trace!("Calculated hash {}", color::symbol(&hash));

    Ok(hash)
}
