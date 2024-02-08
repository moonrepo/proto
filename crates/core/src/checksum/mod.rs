mod minisign;
mod sha256;

use crate::error::ProtoError;
use std::path::Path;

pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: Option<&str>,
) -> miette::Result<bool> {
    match checksum_file.extension().map(|e| e.to_str().unwrap()) {
        Some("minisig" | "minisign") => minisign::verify_checksum(
            download_file,
            checksum_file,
            checksum_public_key.ok_or_else(|| ProtoError::MissingChecksumPublicKey)?,
        ),
        _ => sha256::verify_checksum(download_file, checksum_file),
    }
}
