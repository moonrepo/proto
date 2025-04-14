mod checksum_error;
mod minisign;
mod sha256;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

pub use checksum_error::*;

#[tracing::instrument(skip_all)]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: Option<&str>,
) -> miette::Result<bool> {
    match checksum_file.extension().and_then(|ext| ext.to_str()) {
        Some("minisig" | "minisign") => minisign::verify_checksum(
            download_file,
            checksum_file,
            checksum_public_key.ok_or(ProtoChecksumError::MissingChecksumPublicKey)?,
        ),
        _ => sha256::verify_checksum(download_file, checksum_file),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(into = "String", try_from = "String")]
pub enum ChecksumRecord {
    Minisign(String),
    Sha256(String),
}

impl FromStr for ChecksumRecord {
    type Err = ProtoChecksumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once(':') {
            Some((pre, suf)) => match pre {
                "minisign" => Ok(Self::Minisign(suf.to_owned())),
                "sha256" => Ok(Self::Sha256(suf.to_owned())),
                _ => Err(ProtoChecksumError::UnknownChecksumType {
                    kind: pre.to_owned(),
                }),
            },
            None => Err(ProtoChecksumError::MissingChecksumType),
        }
    }
}

impl TryFrom<String> for ChecksumRecord {
    type Error = ProtoChecksumError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl fmt::Display for ChecksumRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minisign(hash) => write!(f, "minisign:{hash}"),
            Self::Sha256(hash) => write!(f, "sha256:{hash}"),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for ChecksumRecord {
    fn into(self) -> String {
        self.to_string()
    }
}
