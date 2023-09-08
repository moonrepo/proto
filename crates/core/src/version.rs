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
pub enum UnresolvedVersionSpec {
    Canary,
    Alias(String),
    Req(VersionReq),
    ReqAny(Vec<VersionReq>),
    Version(Version),
}

impl UnresolvedVersionSpec {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        Ok(Self::from_str(value.as_ref())?)
    }

    pub fn is_canary(&self) -> bool {
        matches!(self, UnresolvedVersionSpec::Canary)
    }

    pub fn to_spec(&self) -> VersionSpec {
        match self {
            UnresolvedVersionSpec::Canary => VersionSpec::Alias("canary".to_owned()),
            UnresolvedVersionSpec::Alias(alias) => VersionSpec::Alias(alias.to_owned()),
            UnresolvedVersionSpec::Version(version) => VersionSpec::Version(version.to_owned()),
            _ => unreachable!(),
        }
    }
}

impl Default for UnresolvedVersionSpec {
    fn default() -> Self {
        Self::Alias("latest".into())
    }
}

impl FromStr for UnresolvedVersionSpec {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = remove_space_after_gtlt(remove_v_prefix(value.trim().replace(".*", "")));

        if value == "canary" {
            return Ok(UnresolvedVersionSpec::Canary);
        }

        if is_alias_name(&value) {
            return Ok(UnresolvedVersionSpec::Alias(value));
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

            return Ok(UnresolvedVersionSpec::ReqAny(any));
        }

        // AND requirements
        if value.contains(',') {
            return Ok(UnresolvedVersionSpec::Req(
                VersionReq::parse(&value).map_err(handle_error)?,
            ));
        } else if value.contains(' ') {
            return Ok(UnresolvedVersionSpec::Req(
                VersionReq::parse(&value.replace(' ', ", ")).map_err(handle_error)?,
            ));
        }

        Ok(match value.chars().next().unwrap() {
            '=' | '^' | '~' | '>' | '<' | '*' => {
                UnresolvedVersionSpec::Req(VersionReq::parse(&value).map_err(handle_error)?)
            }
            _ => {
                let dot_count = value.match_indices('.').collect::<Vec<_>>().len();

                // If not fully qualified, match using a requirement
                if dot_count < 2 {
                    UnresolvedVersionSpec::Req(
                        VersionReq::parse(&format!("~{value}")).map_err(handle_error)?,
                    )
                } else {
                    UnresolvedVersionSpec::Version(Version::parse(&value).map_err(handle_error)?)
                }
            }
        })
    }
}

impl TryFrom<String> for UnresolvedVersionSpec {
    type Error = ProtoError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for UnresolvedVersionSpec {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Display for UnresolvedVersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canary => write!(f, "canary"),
            Self::Alias(alias) => write!(f, "{}", alias),
            Self::Req(req) => write!(f, "{}", req),
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

impl PartialEq<VersionSpec> for UnresolvedVersionSpec {
    fn eq(&self, other: &VersionSpec) -> bool {
        match (self, other) {
            (Self::Canary, VersionSpec::Alias(a)) => a == "canary",
            (Self::Alias(a1), VersionSpec::Alias(a2)) => a1 == a2,
            (Self::Version(v1), VersionSpec::Version(v2)) => v1 == v2,
            _ => false,
        }
    }
}

#[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum VersionSpec {
    Alias(String),
    Version(Version),
}

impl VersionSpec {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        Ok(Self::from_str(value.as_ref())?)
    }

    pub fn is_canary(&self) -> bool {
        match self {
            Self::Alias(alias) => alias == "canary",
            Self::Version(_) => false,
        }
    }

    pub fn is_latest(&self) -> bool {
        match self {
            Self::Alias(alias) => alias == "latest",
            Self::Version(_) => false,
        }
    }

    pub fn to_unresolved_spec(&self) -> UnresolvedVersionSpec {
        match self {
            Self::Alias(alias) => UnresolvedVersionSpec::Alias(alias.to_owned()),
            Self::Version(version) => UnresolvedVersionSpec::Version(version.to_owned()),
        }
    }
}

impl Default for VersionSpec {
    fn default() -> Self {
        Self::Alias("latest".into())
    }
}

impl FromStr for VersionSpec {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = remove_space_after_gtlt(remove_v_prefix(value.trim().replace(".*", "")));

        if is_alias_name(&value) {
            return Ok(VersionSpec::Alias(value));
        }

        Ok(VersionSpec::Version(Version::parse(&value).map_err(
            |error| ProtoError::Semver {
                version: value,
                error,
            },
        )?))
    }
}

impl TryFrom<String> for VersionSpec {
    type Error = ProtoError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for VersionSpec {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Debug for VersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for VersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alias(alias) => write!(f, "{}", alias),
            Self::Version(version) => write!(f, "{}", version),
        }
    }
}

impl PartialEq<&str> for VersionSpec {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Self::Alias(alias) => alias == other,
            Self::Version(version) => version.to_string() == *other,
        }
    }
}

impl PartialEq<Version> for VersionSpec {
    fn eq(&self, other: &Version) -> bool {
        match self {
            Self::Version(version) => version == other,
            _ => false,
        }
    }
}
