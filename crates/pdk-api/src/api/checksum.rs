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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChecksumAlgorithm {
    Gpg,
    Minisign,
    Sha256,
    Sha512,
}

/// Represents a checksum for a specific algorithm.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct Checksum {
    /// Algorithm.
    pub algo: ChecksumAlgorithm,

    /// Public key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// File hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl Checksum {
    pub fn gpg(key: String) -> Self {
        Self {
            algo: ChecksumAlgorithm::Gpg,
            key: Some(key),
            hash: None,
        }
    }

    /// Create a compact record of a successfully verified GPG signature.
    pub fn gpg_verified(fingerprint: String, sha256: String) -> Self {
        Self {
            algo: ChecksumAlgorithm::Gpg,
            key: Some(fingerprint),
            hash: Some(sha256),
        }
    }

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
                "gpg" => {
                    if let Some((fingerprint, sha256)) = hash.split_once(":sha256:")
                        && matches!(fingerprint.len(), 32 | 40 | 64)
                        && fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit())
                        && sha256.len() == 64
                        && sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                    {
                        Ok(Self::gpg_verified(
                            fingerprint.to_owned(),
                            sha256.to_owned(),
                        ))
                    } else {
                        Ok(Self::gpg(hash.to_owned()))
                    }
                }
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
            ChecksumAlgorithm::Gpg => {
                write!(f, "gpg:{}", self.key.as_deref().unwrap_or_default())?;

                if let Some(hash) = &self.hash {
                    write!(f, ":sha256:{hash}")?;
                }

                Ok(())
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_verified_gpg_checksum() {
        let fingerprint = "0123456789ABCDEF0123456789ABCDEF01234567";
        let hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let checksum = Checksum::gpg_verified(fingerprint.into(), hash.into());

        assert_eq!(checksum.to_string().parse::<Checksum>().unwrap(), checksum);
    }

    #[test]
    fn preserves_armored_gpg_public_key() {
        let key = "-----BEGIN PGP PUBLIC KEY BLOCK-----\ncomment:sha256:not-a-hash\n-----END PGP PUBLIC KEY BLOCK-----";
        let checksum = Checksum::gpg(key.into());

        assert_eq!(checksum.to_string().parse::<Checksum>().unwrap(), checksum);
    }
}
