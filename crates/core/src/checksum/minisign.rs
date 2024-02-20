use crate::error::ProtoError;
use minisign_verify::*;
use starbase_utils::fs;
use std::path::Path;

pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: &str,
) -> miette::Result<bool> {
    let handle_error = |error: Error| ProtoError::Minisign { error };

    PublicKey::from_base64(checksum_public_key)
        .map_err(handle_error)?
        .verify(
            &fs::read_file_bytes(download_file)?,
            &Signature::decode(&fs::read_file(checksum_file)?).map_err(handle_error)?,
            false,
        )
        .map_err(handle_error)?;

    Ok(true)
}
