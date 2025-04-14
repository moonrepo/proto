use crate::checksum::ProtoChecksumError;
use crate::tool_spec::Backend;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use version_spec::VersionSpec;

pub const LOCKFILE_NAME: &str = "lockfile.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(into = "String", try_from = "String")]
pub enum LockfileChecksum {
    Minisign(String),
    Sha256(String),
}

impl FromStr for LockfileChecksum {
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

impl TryFrom<String> for LockfileChecksum {
    type Error = ProtoChecksumError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl fmt::Display for LockfileChecksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minisign(hash) => write!(f, "minisign:{hash}"),
            Self::Sha256(hash) => write!(f, "sha256:{hash}"),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for LockfileChecksum {
    fn into(self) -> String {
        self.to_string()
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LockfileRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<LockfileChecksum>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lockfile {
    pub installed: BTreeMap<VersionSpec, LockfileRecord>,
}
