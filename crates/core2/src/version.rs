use crate::error::ProtoError;
use crate::helpers::is_alias_name;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum VersionType {
    Alias(String),
    ReqAll(VersionReq),
    ReqAny(Vec<VersionReq>),
    Version(Version),
}

impl FromStr for VersionType {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();

        if is_alias_name(value) {
            return Ok(VersionType::Alias(value.to_owned()));
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

            return Ok(VersionType::ReqAny(any));
        }

        // AND requirements
        if value.contains(',') {
            return Ok(VersionType::ReqAll(
                VersionReq::parse(value).map_err(handle_error)?,
            ));
        }

        Ok(match value.chars().next().unwrap() {
            '=' | '^' | '~' | '>' | '<' | '*' => {
                VersionType::ReqAll(VersionReq::parse(value).map_err(handle_error)?)
            }
            _ => VersionType::Version(Version::parse(value).map_err(handle_error)?),
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
        let value = value.trim();

        if is_alias_name(value) {
            return Ok(AliasOrVersion::Alias(value.to_owned()));
        }

        Ok(AliasOrVersion::Version(Version::parse(value).map_err(
            |error| ProtoError::Semver {
                version: value.to_owned(),
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
