#![allow(clippy::from_over_into)]

use crate::spec_error::SpecError;
use crate::unresolved_parser::*;
use crate::version_types::*;
use crate::{VersionSpec, clean_version_req_string, clean_version_string, is_alias_name};
use compact_str::CompactString;
use human_sort::compare;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Represents an unresolved version or alias that must be resolved
/// to a fully-qualified version.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum UnresolvedVersionSpec {
    /// A special canary target.
    Canary,
    /// An alias that is used as a map to a version.
    Alias(CompactString),
    /// A partial version, requirement, or range (`^`, `~`, etc).
    Req(VersionReq),
    /// A list of requirements to match any against (joined by `||`).
    ReqAny(Vec<VersionReq>),
    /// A fully-qualified calendar version.
    Calendar(CalVer),
    /// A fully-qualified semantic version.
    Semantic(SemVer),
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
    /// - Else parse as `Semantic` or `Calendar` types.
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, SpecError> {
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
    /// Furthermore, the `Req` and `ReqAny` variants will return a "latest" alias,
    ///  as they are not resolved or valid versions.
    pub fn to_resolved_spec(&self) -> VersionSpec {
        match self {
            Self::Canary => VersionSpec::Canary,
            Self::Alias(alias) => VersionSpec::Alias(CompactString::new(alias)),
            Self::Calendar(version) => VersionSpec::Calendar(version.to_owned()),
            Self::Semantic(version) => VersionSpec::Semantic(version.to_owned()),
            _ => VersionSpec::default(),
        }
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for UnresolvedVersionSpec {
    fn schema_name() -> Option<String> {
        Some("UnresolvedVersionSpec".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.set_description("Represents an unresolved version or alias that must be resolved to a fully-qualified version.");
        schema.string_default()
    }
}

impl Default for UnresolvedVersionSpec {
    /// Returns a `latest` alias.
    fn default() -> Self {
        Self::Alias("latest".into())
    }
}

impl FromStr for UnresolvedVersionSpec {
    type Err = SpecError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "canary" {
            return Ok(UnresolvedVersionSpec::Canary);
        }

        let value = clean_version_string(value);

        if is_alias_name(&value) {
            return Ok(UnresolvedVersionSpec::Alias(CompactString::new(value)));
        }

        let value = clean_version_req_string(&value);

        // OR requirements
        if value.contains("||") {
            let mut reqs = vec![];

            for result in parse_multi(&value)? {
                reqs.push(VersionReq::parse(&result)?);
            }

            return Ok(UnresolvedVersionSpec::ReqAny(reqs));
        }

        // Version or requirement
        let (result, kind) = parse(value)?;

        Ok(match kind {
            ParseKind::Req => UnresolvedVersionSpec::Req(VersionReq::parse(&result)?),
            ParseKind::Cal => UnresolvedVersionSpec::Calendar(CalVer::parse(&result)?),
            _ => UnresolvedVersionSpec::Semantic(SemVer::parse(&result)?),
        })
    }
}

impl TryFrom<String> for UnresolvedVersionSpec {
    type Error = SpecError;

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
            Self::Alias(alias) => write!(f, "{alias}"),
            Self::Req(req) => write!(f, "{req}"),
            Self::ReqAny(reqs) => write!(
                f,
                "{}",
                reqs.iter()
                    .map(|req| req.to_string())
                    .collect::<Vec<_>>()
                    .join(" || ")
            ),
            Self::Calendar(version) => write!(f, "{version}"),
            Self::Semantic(version) => write!(f, "{version}"),
        }
    }
}

impl PartialEq<VersionSpec> for UnresolvedVersionSpec {
    fn eq(&self, other: &VersionSpec) -> bool {
        match (self, other) {
            (Self::Canary, VersionSpec::Alias(a)) => a == "canary",
            (Self::Alias(a1), VersionSpec::Alias(a2)) => a1 == a2,
            (Self::Calendar(v1), VersionSpec::Calendar(v2)) => v1 == v2,
            (Self::Semantic(v1), VersionSpec::Semantic(v2)) => v1 == v2,
            _ => false,
        }
    }
}

impl AsRef<UnresolvedVersionSpec> for UnresolvedVersionSpec {
    fn as_ref(&self) -> &UnresolvedVersionSpec {
        self
    }
}

impl PartialOrd<UnresolvedVersionSpec> for UnresolvedVersionSpec {
    fn partial_cmp(&self, other: &UnresolvedVersionSpec) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UnresolvedVersionSpec {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Canary, Self::Canary) => Ordering::Equal,
            (Self::Alias(l), Self::Alias(r)) => l.cmp(r),
            (Self::Calendar(l), Self::Calendar(r)) => l.cmp(r),
            (Self::Semantic(l), Self::Semantic(r)) => l.cmp(r),
            _ => compare(&self.to_string(), &other.to_string()),
        }
    }
}
