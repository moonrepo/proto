#![allow(clippy::from_over_into)]

use crate::{clean_version_string, is_alias_name, VersionSpec};
use human_sort::compare;
use semver::{Error, Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Represents an unresolved version or alias that must be resolved
/// to a fully-qualified and semantic result.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum UnresolvedVersionSpec {
    /// A special canary target.
    Canary,
    /// An alias that is used as a map to a version.
    Alias(String),
    /// A partial version, requirement, or range (`^`, `~`, etc).
    Req(VersionReq),
    /// A list of requirements to match any against (joined by `||`).
    ReqAny(Vec<VersionReq>),
    /// A fully-qualified semantic version.
    Version(Version),
}

impl UnresolvedVersionSpec {
    /// Parse the provided string into an unresolved specification based
    /// on the following rules, in order:
    ///
    /// - If the value "canary", map as `Canary` variant.
    /// - If an alpha-numeric value that starts with a character, map as `Alias`.
    /// - If contains `||`, split and parse each item with [`VersionReq`],
    ///   and map as `ReqAny`.
    /// - If contains `,` or ` ` (space), parse with [`VersionReq`], and map as `Req`.
    /// - If starts with `=`, `^`, `~`, `>`, `<`, or `*`, parse with [`VersionReq`],
    ///   and map as `Req`.
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

    /// Convert the current unresolved specification to a resolved specification.
    /// Note that this *does not* actually resolve or validate against a manifest,
    /// and instead simply constructs the [`VersionSpec`].
    ///
    /// Furthermore, the `Req` and `ReqAny` variants will panic, as they are not
    /// resolved or valid versions.
    pub fn to_resolved_spec(&self) -> VersionSpec {
        match self {
            Self::Canary => VersionSpec::Canary,
            Self::Alias(alias) => VersionSpec::Alias(alias.to_owned()),
            Self::Version(version) => VersionSpec::Version(version.to_owned()),
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for UnresolvedVersionSpec {
    fn generate_schema() -> schematic::SchemaType {
        schematic::SchemaType::string()
    }
}

impl Default for UnresolvedVersionSpec {
    /// Returns a `latest` alias.
    fn default() -> Self {
        Self::Alias("latest".into())
    }
}

impl FromStr for UnresolvedVersionSpec {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "canary" {
            return Ok(UnresolvedVersionSpec::Canary);
        }

        let value = clean_version_string(value);

        if is_alias_name(&value) {
            return Ok(UnresolvedVersionSpec::Alias(value));
        }

        // OR requirements (Node.js)
        if value.contains("||") {
            let mut any = vec![];
            let mut parts = value.split("||").map(|p| p.trim()).collect::<Vec<_>>();

            // Try and sort from highest to lowest range
            parts.sort_by(|a, d| compare(d, a));

            for req in parts {
                any.push(VersionReq::parse(req)?);
            }

            return Ok(UnresolvedVersionSpec::ReqAny(any));
        }

        // AND requirements
        if value.contains(',') {
            return Ok(UnresolvedVersionSpec::Req(VersionReq::parse(&value)?));
        }

        Ok(match value.chars().next().unwrap() {
            '=' | '^' | '~' | '>' | '<' | '*' => {
                UnresolvedVersionSpec::Req(VersionReq::parse(&value)?)
            }
            _ => {
                let dot_count = value.match_indices('.').collect::<Vec<_>>().len();

                // If not fully qualified, match using a requirement
                if dot_count < 2 {
                    UnresolvedVersionSpec::Req(VersionReq::parse(&format!("~{value}"))?)
                } else {
                    UnresolvedVersionSpec::Version(Version::parse(&value)?)
                }
            }
        })
    }
}

impl TryFrom<String> for UnresolvedVersionSpec {
    type Error = Error;

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

impl AsRef<UnresolvedVersionSpec> for UnresolvedVersionSpec {
    fn as_ref(&self) -> &UnresolvedVersionSpec {
        self
    }
}
