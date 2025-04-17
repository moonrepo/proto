mod checksum_error;
mod minisign;
mod sha256;

use crate::lockfile::ChecksumRecord;
use std::path::Path;

pub use checksum_error::*;
pub use sha256::hash_file_contents;

#[tracing::instrument(skip_all)]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: Option<&str>,
) -> miette::Result<Option<ChecksumRecord>> {
    match checksum_file.extension().and_then(|ext| ext.to_str()) {
        Some("minisig" | "minisign") => minisign::verify_checksum(
            download_file,
            checksum_file,
            checksum_public_key.ok_or(ProtoChecksumError::MissingChecksumPublicKey)?,
        ),
        _ => sha256::verify_checksum(download_file, checksum_file),
    }
}
