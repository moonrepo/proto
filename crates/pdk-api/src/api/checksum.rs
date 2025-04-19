use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use std::str::FromStr;
use thiserror::Error;

/// Errors that may occur from within a plugin.
#[derive(Error, Debug)]
pub enum ChecksumError {
    #[error("Checksum type is not defined.")]
    MissingChecksumType,

    #[error("Unknown or unsupported checksum type {kind}.")]
    UnknownChecksumType { kind: String },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub enum Checksum {
    Minisign(String),
    Sha256(String),
}

impl Checksum {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Minisign(hash) => hash,
            Self::Sha256(hash) => hash,
        }
    }
}

impl FromStr for Checksum {
    type Err = ChecksumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split_once(':') {
            Some((tag, hash)) => match tag {
                "minisign" => Ok(Self::Minisign(hash.to_owned())),
                "sha256" => Ok(Self::Sha256(hash.to_owned())),
                _ => Err(ChecksumError::UnknownChecksumType {
                    kind: tag.to_owned(),
                }),
            },
            None => Err(ChecksumError::MissingChecksumType),
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
        match self {
            Self::Minisign(hash) => write!(f, "minisign:{hash}"),
            Self::Sha256(hash) => write!(f, "sha256:{hash}"),
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
