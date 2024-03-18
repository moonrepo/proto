use sha2::{Digest, Sha256};
use starbase_utils::fs::{self, FsError};
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::trace;

pub fn hash_file_contents<P: AsRef<Path>>(path: P) -> miette::Result<String> {
    let path = path.as_ref();

    trace!(file = ?path, "Calculating SHA256 checksum");

    let mut file = fs::open_file(path)?;
    let mut sha = Sha256::new();

    io::copy(&mut file, &mut sha).map_err(|error| FsError::Read {
        path: path.to_path_buf(),
        error,
    })?;

    let hash = format!("{:x}", sha.finalize());

    trace!(hash, "Calculated hash");

    Ok(hash)
}

pub fn verify_checksum(download_file: &Path, checksum_file: &Path) -> miette::Result<bool> {
    let checksum_hash = hash_file_contents(download_file)?;
    let download_file_name = fs::file_name(download_file);

    for line in BufReader::new(fs::open_file(checksum_file)?)
        .lines()
        .map_while(Result::ok)
    {
        // <checksum>  <file>
        // <checksum> *<file>
        // <checksum>
        if line == checksum_hash
            || (line.starts_with(&checksum_hash) && line.ends_with(&download_file_name))
        {
            return Ok(true);
        }
    }

    Ok(false)
}
