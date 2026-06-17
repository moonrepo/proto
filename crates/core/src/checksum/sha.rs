use super::checksum_error::ProtoChecksumError;
use starbase_utils::fs;
use starbase_utils::hash::{self, HashError};
use std::fmt::Debug;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::{instrument, trace};

#[instrument]
pub fn hash_file_contents_sha256<P: AsRef<Path> + Debug>(path: P) -> Result<String, HashError> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA256 checksum");

    let hash = hash::sha256::from_file(path)?;

    trace!(file = ?path, hash, "Calculated hash");

    Ok(hash)
}

#[instrument]
pub fn hash_file_contents_sha512<P: AsRef<Path> + Debug>(path: P) -> Result<String, HashError> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA512 checksum");

    let hash = hash::sha512::from_file(path)?;

    trace!(file = ?path, hash, "Calculated hash");

    Ok(hash)
}

#[instrument(name = "verify_sha_checksum")]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_hash: &str,
) -> Result<bool, ProtoChecksumError> {
    let download_file_name = fs::file_name(download_file);

    for line in BufReader::new(fs::open_file(checksum_file)?)
        .lines()
        .map_while(Result::ok)
    {
        if line.is_empty() {
            continue;
        }

        // <checksum>  <file>
        // <checksum> *<file>
        // <checksum>
        if line == checksum_hash
            || (line.starts_with(checksum_hash) && line.ends_with(&download_file_name))
        {
            return Ok(true);
        }

        // Checksum files on Windows are created with Get-FileHash,
        // which has a different file structure than Unix
        // https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/get-filehash?view=powershell-7.5
        if line.starts_with("Hash")
            && let Some((_, hash)) = line.split_once(':')
        {
            // The hash is all uppercase in the checksum file,
            // but the one's we generate are not, so lowercase
            return Ok(hash.trim().to_lowercase() == checksum_hash);
        }
    }

    Ok(false)
}
