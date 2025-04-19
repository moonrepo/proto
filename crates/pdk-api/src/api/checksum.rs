use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use std::str::FromStr;
use thiserror::Error;

/// Errors that may occur from within a plugin.
#[derive(Error, Debug)]
pub enum ChecksumError {
    #[error("Checksum algorithm is not defined.")]
    MissingAlgorithm,

    #[error("Unknown or unsupported checksum algorithm {kind}.")]
    UnsupportedAlgorithm { kind: String },
}

/// Supported checksum algorithms.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumAlgorithm {
    Minisign,
    Sha256,
    Sha512,
}

/// Represents a checksum for a specific algorithm.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct Checksum {
    /// Algorithm.
    pub algo: ChecksumAlgorithm,

    /// Public key.
    pub key: Option<String>,

    /// File hash.
    pub hash: Option<String>,
}

impl Checksum {
    pub fn minisign(key: String) -> Self {
        Self {
            algo: ChecksumAlgorithm::Minisign,
            key: Some(key),
            hash: None,
        }
    }

    pub fn sha256(hash: String) -> Self {
        Self {
            algo: ChecksumAlgorithm::Sha256,
            key: None,
            hash: Some(hash),
        }
    }

    pub fn sha512(hash: String) -> Self {
        Self {
            algo: ChecksumAlgorithm::Sha512,
            key: None,
            hash: Some(hash),
        }
    }
}

impl FromStr for Checksum {
    type Err = ChecksumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if !value.contains(':') {
            if value.len() == 64 {
                return Ok(Self::sha256(value.to_owned()));
            } else if value.len() == 128 {
                return Ok(Self::sha512(value.to_owned()));
            }
        }

        match value.split_once(':') {
            Some((tag, hash)) => match tag {
                "minisign" => Ok(Self::minisign(hash.to_owned())),
                "sha256" => Ok(Self::sha256(hash.to_owned())),
                "sha512" => Ok(Self::sha512(hash.to_owned())),
                _ => Err(ChecksumError::UnsupportedAlgorithm {
                    kind: tag.to_owned(),
                }),
            },
            None => Err(ChecksumError::MissingAlgorithm),
        }
    }
}

impl TryFrom<String> for Checksum {
    type Error = ChecksumError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.algo {
            ChecksumAlgorithm::Minisign => {
                write!(f, "minisign:{}", self.key.as_deref().unwrap_or_default())
            }
            ChecksumAlgorithm::Sha256 => {
                write!(f, "sha256:{}", self.hash.as_deref().unwrap_or_default())
            }
            ChecksumAlgorithm::Sha512 => {
                write!(f, "sha512:{}", self.hash.as_deref().unwrap_or_default())
            }
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for Checksum {
    fn into(self) -> String {
        self.to_string()
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for Checksum {
    fn schema_name() -> Option<String> {
        Some("Checksum".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}
