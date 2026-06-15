use super::checksum_error::ProtoChecksumError;
use sha2::{Digest, Sha256, Sha512};
use starbase_utils::fs::{self, FsError};
use std::fmt::Debug;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use tracing::{instrument, trace};

#[instrument]
pub fn hash_file_contents_sha256<P: AsRef<Path> + Debug>(path: P) -> Result<String, FsError> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA256 checksum");

    let mut sha = Sha256::new();
    let mut file = fs::open_file(path)?;
    let mut buffer = [0u8; 64 * 1024];

    // Read in chunks instead of pulling the entire file into memory
    loop {
        let n = file.read(&mut buffer).map_err(|error| FsError::Read {
            path: path.to_path_buf(),
            error: Box::new(error),
        })?;

        if n == 0 {
            break;
        }

        sha.update(&buffer[..n]);
    }

    let hash = hex::encode(sha.finalize());

    trace!(file = ?path, hash, "Calculated hash");

    Ok(hash)
}

#[instrument]
pub fn hash_file_contents_sha512<P: AsRef<Path> + Debug>(path: P) -> Result<String, FsError> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA512 checksum");

    let mut sha = Sha512::new();
    let mut file = fs::open_file(path)?;
    let mut buffer = [0u8; 64 * 1024];

    // Read in chunks instead of pulling the entire file into memory
    loop {
        let n = file.read(&mut buffer).map_err(|error| FsError::Read {
            path: path.to_path_buf(),
            error: Box::new(error),
        })?;

        if n == 0 {
            break;
        }

        sha.update(&buffer[..n]);
    }

    let hash = hex::encode(sha.finalize());

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

    for line in
        BufReader::new(
            fs::open_file(checksum_file).map_err(|error| ProtoChecksumError::Sha {
                error: Box::new(error),
            })?,
        )
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
