mod checksum_error;
mod minisign;
mod sha;

use proto_pdk_api::Checksum;
use starbase_utils::fs;
use std::path::Path;

pub use checksum_error::*;
pub use sha::{hash_file_contents_sha256, hash_file_contents_sha512};

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
        Checksum::Sha256(hash) | Checksum::Sha512(hash) => {
            sha::verify_checksum(download_file, checksum_file, hash)
        }
    }
}

#[tracing::instrument(skip_all)]
pub fn generate_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: Option<&str>,
) -> miette::Result<Checksum> {
    match detect_checksum_algorithm(checksum_file)?.as_str() {
        "minisign" => Ok(Checksum::Minisign(
            checksum_public_key
                .ok_or(ProtoChecksumError::MissingPublicKey)?
                .to_owned(),
        )),
        "sha512" => Ok(Checksum::Sha512(hash_file_contents_sha512(download_file)?)),
        _ => Ok(Checksum::Sha256(hash_file_contents_sha256(download_file)?)),
    }
}

#[tracing::instrument(skip_all)]
pub fn detect_checksum_algorithm(checksum_file: &Path) -> miette::Result<String> {
    // Check file extension
    let mut algo = match checksum_file.extension().and_then(|ext| ext.to_str()) {
        Some("minisig" | "minisign") => "minisign",
        Some("sha256" | "sha256sum") => "sha256",
        Some("sha512" | "sha512sum") => "sha512",
        _ => "",
    }
    .to_owned();

    // Then check file name
    if algo.is_empty() {
        algo = match fs::file_name(checksum_file).to_lowercase().as_str() {
            "shasums256.txt" => "sha256",
            "shasums512.txt" => "sha512",
            _ => "",
        }
        .to_owned();
    }

    // Then check the file contents
    if algo.is_empty() {
        let contents = fs::read_file(checksum_file)?;

        for line in contents.lines() {
            if line.is_empty() {
                continue;
            }

            // Windows
            if line.contains(':') {
                if let Some((label, value)) = line.split_once(':') {
                    if label.trim() == "Algorithm" {
                        algo = value.trim().to_owned();
                        break;
                    }
                }

                continue;
            }

            // Unix
            if let Some((hash, _)) = line.split_once("  ") {
                if hash.len() == 64 {
                    algo = "sha256".to_owned();
                } else if hash.len() == 128 {
                    algo = "sha512".to_owned();
                }

                break;
            }
        }
    }

    if algo == "minisign" || algo == "sha256" || algo == "sha512" {
        return Ok(algo);
    }

    Err(ProtoChecksumError::UnsupportedAlgorithm { algo }.into())
}
