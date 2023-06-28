use crate::WasmPlugin;
use proto_core::{
    async_trait, get_sha256_hash_of_file, Describable, ProtoError, Resolvable, Verifiable,
};
use proto_pdk::{VerifyChecksumInput, VerifyChecksumOutput};
use starbase_utils::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tracing::debug;

#[async_trait]
impl Verifiable<'_> for WasmPlugin {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(self.get_resolved_version())
            .join("CHECKSUM.txt"))
    }

    fn get_checksum_url(&self) -> Result<Option<String>, ProtoError> {
        Ok(self.get_install_params()?.checksum_url)
    }

    async fn verify_checksum(
        &self,
        checksum_file: &Path,
        download_file: &Path,
    ) -> Result<bool, ProtoError> {
        if !checksum_file.exists() {
            return Ok(true);
        }

        debug!(
            tool = self.get_id(),
            download_file = ?download_file,
            checksum_file = ?checksum_file,
            "Verifiying checksum of downloaded file",
        );

        // Allow plugin to provide their own checksum verification method
        if self.has_func("verify_checksum") {
            let params: VerifyChecksumOutput = self.call_func_with(
                "verify_checksum",
                VerifyChecksumInput {
                    checksum_file: self.to_wasi_virtual_path(checksum_file),
                    download_file: self.to_wasi_virtual_path(download_file),
                    env: self.get_environment(),
                },
            )?;

            if params.verified {
                return Ok(true);
            }

            return Err(ProtoError::VerifyInvalidChecksum(
                download_file.to_path_buf(),
                checksum_file.to_path_buf(),
            ));
        }

        // Otherwise attempt to verify it ourselves
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
                debug!(
                    tool = self.get_id(),
                    "Successfully verified, checksum matches"
                );

                return Ok(true);
            }
        }

        Err(ProtoError::VerifyInvalidChecksum(
            download_file.to_path_buf(),
            checksum_file.to_path_buf(),
        ))
    }
}
