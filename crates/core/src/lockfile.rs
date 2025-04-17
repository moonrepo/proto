use crate::checksum::ProtoChecksumError;
use crate::tool_spec::Backend;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockfileRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,

    pub checksum: ChecksumRecord,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

impl Default for LockfileRecord {
    fn default() -> Self {
        Self {
            backend: None,
            checksum: ChecksumRecord::Sha256("unknown".into()),
            source: None,
            suffix: None,
        }
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
