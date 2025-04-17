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
) -> miette::Result<Option<ChecksumRecord>> {
    match checksum_file.extension().and_then(|ext| ext.to_str()) {
        Some("minisig" | "minisign") => minisign::verify_checksum(
            download_file,
            checksum_file,
            checksum_public_key.ok_or(ProtoChecksumError::MissingChecksumPublicKey)?,
        ),
        _ => sha256::verify_checksum(download_file, checksum_file),
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub enum ChecksumRecord {
    Minisign(String),
    Sha256(String),
}

impl ChecksumRecord {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Minisign(hash) => hash,
            Self::Sha256(hash) => hash,
        }
    }
}

impl FromStr for ChecksumRecord {
    type Err = ProtoChecksumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once(':') {
            Some((tag, hash)) => match tag {
                "minisign" => Ok(Self::Minisign(hash.to_owned())),
                "sha256" => Ok(Self::Sha256(hash.to_owned())),
                _ => Err(ProtoChecksumError::UnknownChecksumType {
                    kind: tag.to_owned(),
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
