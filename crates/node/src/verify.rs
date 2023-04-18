use crate::NodeLanguage;
use proto_core::{async_trait, get_sha256_hash_of_file, ProtoError, Resolvable, Verifiable};
use starbase_utils::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tracing::debug;

#[async_trait]
impl Verifiable<'_> for NodeLanguage {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(format!("v{}-SHASUMS256.txt", self.get_resolved_version())))
    }

    fn get_checksum_url(&self) -> Result<Option<String>, ProtoError> {
        Ok(Some(format!(
            "https://nodejs.org/dist/v{}/SHASUMS256.txt",
            self.get_resolved_version()
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
        let file_name = fs::file_name(download_file);

        for line in BufReader::new(file).lines().flatten() {
            // <checksum>  node-v<version>-<os>-<arch>.tar.gz
            if line.starts_with(&checksum) && line.ends_with(&file_name) {
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
