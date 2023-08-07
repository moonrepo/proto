#![allow(clippy::from_over_into)]

use crate::error::ProtoError;
use crate::helpers::{is_alias_name, remove_space_after_gtlt, remove_v_prefix};
use human_sort::compare;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum VersionType {
    Alias(String),
    ReqAll(VersionReq),
    ReqAny(Vec<VersionReq>),
    Version(Version),
}

impl VersionType {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        Ok(Self::from_str(value.as_ref())?)
    }

    pub fn to_explicit_version(&self) -> AliasOrVersion {
        match self {
            VersionType::Alias(alias) => AliasOrVersion::Alias(alias.to_owned()),
            VersionType::Version(version) => AliasOrVersion::Version(version.to_owned()),
            _ => unreachable!(),
        }
    }
}

impl Default for VersionType {
    fn default() -> Self {
        Self::Alias("latest".into())
    }
}

impl FromStr for VersionType {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = remove_space_after_gtlt(remove_v_prefix(value.trim().replace(".*", "")));

        if is_alias_name(&value) {
            return Ok(VersionType::Alias(value));
        }

        let handle_error = |error: semver::Error| ProtoError::Semver {
            version: value.to_owned(),
            error,
        };

        // OR requirements
        if value.contains("||") {
            let mut any = vec![];
            let mut parts = value.split("||").map(|p| p.trim()).collect::<Vec<_>>();

            // Try and sort from highest to lowest range
            parts.sort_by(|a, d| compare(d, a));

            for req in parts {
                any.push(VersionReq::parse(req).map_err(handle_error)?);
            }

            return Ok(VersionType::ReqAny(any));
        }

        // AND requirements
        if value.contains(',') {
            return Ok(VersionType::ReqAll(
                VersionReq::parse(&value).map_err(handle_error)?,
            ));
        } else if value.contains(' ') {
            return Ok(VersionType::ReqAll(
                VersionReq::parse(&value.replace(' ', ", ")).map_err(handle_error)?,
            ));
        }

        Ok(match value.chars().next().unwrap() {
            '=' | '^' | '~' | '>' | '<' | '*' => {
                VersionType::ReqAll(VersionReq::parse(&value).map_err(handle_error)?)
            }
            _ => {
                let dot_count = value.match_indices('.').collect::<Vec<_>>().len();

                // If not fully qualified, match using a requirement
                if dot_count < 2 {
                    VersionType::ReqAll(
                        VersionReq::parse(&format!("~{value}")).map_err(handle_error)?,
                    )
                } else {
                    VersionType::Version(Version::parse(&value).map_err(handle_error)?)
                }
            }
        })
    }
}

impl TryFrom<String> for VersionType {
    type Error = ProtoError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for VersionType {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alias(alias) => write!(f, "{}", alias),
            Self::ReqAll(req) => write!(f, "{}", req),
            Self::ReqAny(reqs) => write!(
                f,
                "{}",
                reqs.iter()
                    .map(|req| req.to_string())
                    .collect::<Vec<_>>()
                    .join(" || ")
            ),
            Self::Version(version) => write!(f, "{}", version),
        }
    }
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum AliasOrVersion {
    Alias(String),
    Version(Version),
}

impl AliasOrVersion {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        Ok(Self::from_str(value.as_ref())?)
    }

    pub fn to_implicit_type(&self) -> VersionType {
        match self {
            Self::Alias(alias) => VersionType::Alias(alias.to_owned()),
            Self::Version(version) => VersionType::Version(version.to_owned()),
        }
    }
}

impl Default for AliasOrVersion {
    fn default() -> Self {
        Self::Alias("latest".into())
    }
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
        self.to_string()
    }
}

impl Debug for AliasOrVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for AliasOrVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alias(alias) => write!(f, "{}", alias),
            Self::Version(version) => write!(f, "{}", version),
        }
    }
}

impl PartialEq<&str> for AliasOrVersion {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Self::Alias(alias) => alias == other,
            Self::Version(version) => version.to_string() == *other,
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
