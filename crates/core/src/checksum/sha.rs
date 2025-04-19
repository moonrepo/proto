use sha2::{Digest, Sha256, Sha512};
use starbase_utils::fs::{self, FsError};
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::trace;

pub fn hash_file_contents_sha256<P: AsRef<Path>>(path: P) -> miette::Result<String> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA256 checksum");

    let mut file = fs::open_file(path)?;
    let mut sha = Sha256::new();

    io::copy(&mut file, &mut sha).map_err(|error| FsError::Read {
        path: path.to_path_buf(),
        error: Box::new(error),
    })?;

    let hash = format!("{:x}", sha.finalize());

    trace!(hash, "Calculated hash");

    Ok(hash)
}

pub fn hash_file_contents_sha512<P: AsRef<Path>>(path: P) -> miette::Result<String> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA512 checksum");

    let mut file = fs::open_file(path)?;
    let mut sha = Sha512::new();

    io::copy(&mut file, &mut sha).map_err(|error| FsError::Read {
        path: path.to_path_buf(),
        error: Box::new(error),
    })?;

    let hash = format!("{:x}", sha.finalize());

    trace!(hash, "Calculated hash");

    Ok(hash)
}

#[tracing::instrument(name = "verify_sha_checksum")]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_hash: &str,
) -> miette::Result<bool> {
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
        if line.starts_with("Hash") {
            if let Some((_, hash)) = line.split_once(':') {
                // The hash is all uppercase in the checksum file,
                // but the one's we generate are not, so lowercase
                return Ok(hash.trim().to_lowercase() == checksum_hash);
            }
        }
    }

    Ok(false)
}
