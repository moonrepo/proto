use super::checksum_error::ProtoChecksumError;
use minisign_verify::*;
use proto_pdk_api::Checksum;
use starbase_utils::fs;
use std::path::Path;

#[tracing::instrument(name = "minisign")]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: &str,
) -> miette::Result<Option<Checksum>> {
    let handle_error = |error: Error| ProtoChecksumError::Minisign {
        error: Box::new(error),
    };

    PublicKey::from_base64(checksum_public_key)
        .map_err(handle_error)?
        .verify(
            &fs::read_file_bytes(download_file)?,
            &Signature::decode(&fs::read_file(checksum_file)?).map_err(handle_error)?,
            false,
        )
        .map_err(handle_error)?;

    Ok(Some(Checksum::Minisign(checksum_public_key.to_owned())))
}
