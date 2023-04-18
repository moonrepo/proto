use crate::download::get_archive_file;
use crate::GoLanguage;
use proto_core::{async_trait, get_sha256_hash_of_file, ProtoError, Resolvable, Verifiable};
use starbase_utils::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tracing::debug;

#[async_trait]
impl Verifiable<'_> for GoLanguage {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(format!("v{}-SHASUMS256.txt", self.get_resolved_version())))
    }

    fn get_checksum_url(&self) -> Result<Option<String>, ProtoError> {
        let version = self.get_resolved_version();

        let version = match version.strip_suffix(".0") {
            Some(s) => s,
            None => version,
        };

        Ok(Some(format!(
            "https://dl.google.com/go/{}.sha256",
            get_archive_file(version)?
        )))
    }

    async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
    ) -> Result<bool, ProtoError> {
        debug!(
            download_file = %download_file.display(),
            checksum_file = %checksum_file.display(),
            "Verifiying checksum of downloaded file"
        );

        let checksum = get_sha256_hash_of_file(download_file)?;
        let file = fs::open_file(checksum_file)?;

        for line in BufReader::new(file).lines().flatten() {
            if line.starts_with(&checksum) {
                debug!("Successfully verified, checksum matches");

                return Ok(true);
            }
        }

        Err(ProtoError::VerifyInvalidChecksum(
            download_file.to_path_buf(),
            checksum_file.to_path_buf(),
        ))
    }
}
