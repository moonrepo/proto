mod checksum_error;
mod minisign;
mod sha256;

pub use checksum_error::*;
use std::path::Path;

#[tracing::instrument(skip_all)]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: Option<&str>,
) -> miette::Result<bool> {
    match checksum_file.extension().and_then(|ext| ext.to_str()) {
        Some("minisig" | "minisign") => minisign::verify_checksum(
            download_file,
            checksum_file,
            checksum_public_key.ok_or_else(|| ProtoChecksumError::MissingChecksumPublicKey)?,
        ),
        _ => sha256::verify_checksum(download_file, checksum_file),
    }
}
