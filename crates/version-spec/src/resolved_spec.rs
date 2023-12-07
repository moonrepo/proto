#![allow(clippy::from_over_into)]

use crate::{clean_version_string, is_alias_name, UnresolvedVersionSpec};
use semver::{Error, Version};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Represents a resolved version or alias.
#[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum VersionSpec {
    /// A special canary target.
    Canary,
    /// An alias that is used as a map to a version.
    Alias(String),
    /// A fully-qualified semantic version.
    Version(Version),
}

impl VersionSpec {
    /// Parse the provided string into a resolved specification based
    /// on the following rules, in order:
    ///
    /// - If the value "canary", map as `Canary` variant.
    /// - If an alpha-numeric value that starts with a character, map as `Alias`.
    /// - Else parse with [`Version`], and map as `Version`.
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, Error> {
        Self::from_str(value.as_ref())
    }

    /// Return true if the provided alias matches the current specification.
    pub fn is_alias<A: AsRef<str>>(&self, name: A) -> bool {
        match self {
            Self::Alias(alias) => alias == name.as_ref(),
            _ => false,
        }
    }

    /// Return true if the current specification is canary.
    pub fn is_canary(&self) -> bool {
        match self {
            Self::Canary => true,
            Self::Alias(alias) => alias == "canary",
            _ => false,
        }
    }

    /// Return true if the current specification is the "latest" alias.
    pub fn is_latest(&self) -> bool {
        match self {
            Self::Alias(alias) => alias == "latest",
            _ => false,
        }
    }

    /// Convert the current resolved specification to an unresolved specification.
    pub fn to_unresolved_spec(&self) -> UnresolvedVersionSpec {
        match self {
            Self::Canary => UnresolvedVersionSpec::Canary,
            Self::Alias(alias) => UnresolvedVersionSpec::Alias(alias.to_owned()),
            Self::Version(version) => UnresolvedVersionSpec::Version(version.to_owned()),
        }
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for VersionSpec {
    fn generate_schema() -> schematic::SchemaType {
        schematic::SchemaType::string()
    }
}

impl Default for VersionSpec {
    /// Returns a `latest` alias.
    fn default() -> Self {
        Self::Alias("latest".into())
    }
}

impl FromStr for VersionSpec {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "canary" {
            return Ok(VersionSpec::Canary);
        }

        let value = clean_version_string(value);

        if is_alias_name(&value) {
            return Ok(VersionSpec::Alias(value));
        }

        Ok(VersionSpec::Version(Version::parse(&value)?))
    }
}

impl TryFrom<String> for VersionSpec {
    type Error = Error;

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
        // Debug version as a string instead of a struct
        write!(f, "{}", self)
    }
}

impl Display for VersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canary => write!(f, "canary"),
            Self::Alias(alias) => write!(f, "{}", alias),
            Self::Version(version) => write!(f, "{}", version),
        }
    }
}

impl PartialEq<&str> for VersionSpec {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Self::Canary => "canary" == *other,
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

impl AsRef<VersionSpec> for VersionSpec {
    fn as_ref(&self) -> &VersionSpec {
        self
    }
}
