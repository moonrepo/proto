mod checksum_error;
mod minisign;
mod sha;

use proto_pdk_api::{Checksum, ChecksumAlgorithm};
use starbase_utils::fs;
use std::path::Path;

pub use checksum_error::*;
pub use sha::{hash_file_contents_sha256, hash_file_contents_sha512};

#[tracing::instrument(skip_all)]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum: &Checksum,
) -> Result<bool, ProtoChecksumError> {
    match checksum.algo {
        ChecksumAlgorithm::Minisign => minisign::verify_checksum(
            download_file,
            checksum_file,
            checksum.key.as_deref().unwrap(),
        ),
        ChecksumAlgorithm::Sha256 | ChecksumAlgorithm::Sha512 => sha::verify_checksum(
            download_file,
            checksum_file,
            checksum.hash.as_deref().unwrap(),
        ),
    }
}

#[tracing::instrument(skip_all)]
pub fn generate_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: Option<&str>,
) -> Result<Checksum, ProtoChecksumError> {
    match detect_checksum_algorithm(checksum_file)? {
        ChecksumAlgorithm::Minisign => Ok(Checksum::minisign(
            checksum_public_key
                .ok_or(ProtoChecksumError::MissingPublicKey)?
                .to_owned(),
        )),
        ChecksumAlgorithm::Sha512 => Ok(Checksum::sha512(
            hash_file_contents_sha512(download_file).map_err(|error| ProtoChecksumError::Sha {
                error: Box::new(error),
            })?,
        )),
        _ => Ok(Checksum::sha256(
            hash_file_contents_sha256(download_file).map_err(|error| ProtoChecksumError::Sha {
                error: Box::new(error),
            })?,
        )),
    }
}

#[tracing::instrument(skip_all)]
pub fn detect_checksum_algorithm(
    checksum_file: &Path,
) -> Result<ChecksumAlgorithm, ProtoChecksumError> {
    // Check file extension
    let mut algo = match checksum_file.extension().and_then(|ext| ext.to_str()) {
        Some("minisig" | "minisign") => Some(ChecksumAlgorithm::Minisign),
        Some("sha256" | "sha256sum") => Some(ChecksumAlgorithm::Sha256),
        Some("sha512" | "sha512sum") => Some(ChecksumAlgorithm::Sha512),
        _ => None,
    };

    // Then check file name
    if algo.is_none() {
        algo = match fs::file_name(checksum_file).to_lowercase().as_str() {
            "shasums256.txt" => Some(ChecksumAlgorithm::Sha256),
            "shasums512.txt" => Some(ChecksumAlgorithm::Sha512),
            _ => None,
        };
    }

    // Then check the file contents
    if algo.is_none() {
        let contents = fs::read_file(checksum_file)?;

        for line in contents.lines() {
            if line.is_empty() {
                continue;
            }

            // Windows
            if line.contains(':') {
                if let Some((label, value)) = line.split_once(':') {
                    if label.trim() == "Algorithm" {
                        algo = match value.trim() {
                            "SHA256" => Some(ChecksumAlgorithm::Sha256),
                            "SHA512" => Some(ChecksumAlgorithm::Sha512),
                            other => {
                                return Err(ProtoChecksumError::UnsupportedAlgorithm {
                                    algo: other.into(),
                                }
                                .into());
                            }
                        };

                        break;
                    }
                }

                continue;
            }

            // Unix
            if let Some((hash, _)) = line.split_once("  ") {
                if hash.len() == 64 {
                    algo = Some(ChecksumAlgorithm::Sha256);
                } else if hash.len() == 128 {
                    algo = Some(ChecksumAlgorithm::Sha512);
                }

                break;
            }
        }
    }

    algo.ok_or_else(|| {
        ProtoChecksumError::UnknownAlgorithm {
            path: checksum_file.to_path_buf(),
        }
        .into()
    })
}
