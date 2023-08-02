use crate::error::ProtoError;
use crate::helpers::{is_alias_name, remove_space_after_gtlt, remove_v_prefix};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum DetectedVersion {
    Alias(String),
    ReqAll(VersionReq),
    ReqAny(Vec<VersionReq>),
    Version(Version),
}

impl FromStr for DetectedVersion {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = remove_space_after_gtlt(remove_v_prefix(value.trim().replace(".*", "")));

        if is_alias_name(&value) {
            return Ok(DetectedVersion::Alias(value));
        }

        let handle_error = |error: semver::Error| ProtoError::Semver {
            version: value.to_owned(),
            error,
        };

        // OR requirements
        if value.contains("||") {
            let mut any = vec![];

            for req in value.split("||") {
                any.push(VersionReq::parse(req.trim()).map_err(handle_error)?);
            }

            return Ok(DetectedVersion::ReqAny(any));
        }

        // AND requirements
        if value.contains(',') {
            return Ok(DetectedVersion::ReqAll(
                VersionReq::parse(&value).map_err(handle_error)?,
            ));
        } else if value.contains(' ') {
            return Ok(DetectedVersion::ReqAll(
                VersionReq::parse(&value.replace(' ', ", ")).map_err(handle_error)?,
            ));
        }

        Ok(match value.chars().next().unwrap() {
            '=' | '^' | '~' | '>' | '<' | '*' => {
                DetectedVersion::ReqAll(VersionReq::parse(&value).map_err(handle_error)?)
            }
            _ => {
                let dot_count = value.match_indices('.').collect::<Vec<_>>().len();

                // If not fully qualified, match using a requirement
                if dot_count < 2 {
                    DetectedVersion::ReqAll(
                        VersionReq::parse(&format!("^{value}")).map_err(handle_error)?,
                    )
                } else {
                    DetectedVersion::Version(Version::parse(&value).map_err(handle_error)?)
                }
            }
        })
    }
}

impl TryFrom<String> for DetectedVersion {
    type Error = ProtoError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for DetectedVersion {
    fn into(self) -> String {
        match self {
            Self::Alias(alias) => alias,
            Self::ReqAll(req) => req.to_string(),
            Self::ReqAny(reqs) => reqs
                .into_iter()
                .map(|req| req.to_string())
                .collect::<Vec<_>>()
                .join(" || "),
            Self::Version(version) => version.to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum AliasOrVersion {
    Alias(String),
    Version(Version),
}

impl FromStr for AliasOrVersion {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = remove_space_after_gtlt(remove_v_prefix(value.trim().replace(".*", "")));

        if is_alias_name(&value) {
            return Ok(AliasOrVersion::Alias(value));
        }

        Ok(AliasOrVersion::Version(Version::parse(&value).map_err(
            |error| ProtoError::Semver {
                version: value,
                error,
            },
        )?))
    }
}

impl TryFrom<String> for AliasOrVersion {
    type Error = ProtoError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for AliasOrVersion {
    fn into(self) -> String {
        match self {
            Self::Alias(alias) => alias,
            Self::Version(version) => version.to_string(),
        }
    }
}

impl PartialEq<Version> for AliasOrVersion {
    fn eq(&self, other: &Version) -> bool {
        match self {
            Self::Version(version) => version == other,
            _ => false,
        }
    }
}
