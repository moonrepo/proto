mod checksum_error;
mod minisign;
mod sha256;

use proto_pdk_api::Checksum;
use std::path::Path;

pub use checksum_error::*;
pub use sha256::hash_file_contents;

#[tracing::instrument(skip_all)]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum: &Checksum,
) -> miette::Result<bool> {
    match checksum {
        Checksum::Minisign(public_key) => {
            minisign::verify_checksum(download_file, checksum_file, public_key)
        }
        Checksum::Sha256(hash) => sha256::verify_checksum(download_file, checksum_file, hash),
    }
}

#[tracing::instrument(skip_all)]
pub fn generate_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: Option<&str>,
) -> miette::Result<Checksum> {
    match checksum_file.extension().and_then(|ext| ext.to_str()) {
        Some("minisig" | "minisign") => Ok(Checksum::Minisign(
            checksum_public_key
                .ok_or(ProtoChecksumError::MissingChecksumPublicKey)?
                .to_owned(),
        )),
        _ => Ok(Checksum::Sha256(hash_file_contents(&download_file)?)),
    }
}
