use crate::SchemaPlugin;
use proto_core::{async_trait, color, get_sha256_hash_of_file, ProtoError, Resolvable, Verifiable};
use starbase_utils::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tracing::debug;

#[async_trait]
impl Verifiable<'_> for SchemaPlugin {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(format!("v{}-SHASUMS256.txt", self.get_resolved_version())))
    }

    fn get_checksum_url(&self) -> Result<Option<String>, ProtoError> {
        if let Some(url) = &self.schema.install.checksum_url {
            return Ok(Some(self.format_string(url)));
        }

        Ok(None)
    }

    async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
    ) -> Result<bool, ProtoError> {
        debug!(
            "Verifiying checksum of downloaded file {} using {}",
            color::path(download_file),
            color::path(checksum_file),
        );

        let checksum = get_sha256_hash_of_file(download_file)?;

        let file = fs::open_file(checksum_file)?;
        let file_name = fs::file_name(download_file);

        for line in BufReader::new(file).lines().flatten() {
            if
            // <checksum>  <file>
            line.starts_with(&checksum) && line.ends_with(&file_name) ||
            // <checksum>
            line == checksum
            {
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
