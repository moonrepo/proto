use crate::helpers::hash_file_contents;
use starbase_utils::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

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
