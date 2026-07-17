use crate::resolved_spec::VersionSpec;
use crate::spec_error::SpecError;
use crate::syntax::*;
use crate::syntax_parser::parse_alias;
use compact_str::CompactString;
use human_sort::compare;
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
    /// A list of requirements.
    Range(Range),
    /// A partial version / requirement.
    Requirement(Requirement),
    /// A fully-qualified version.
    Version(Version),
}

impl UnresolvedVersionSpec {
    /// Parse the provided string into an unresolved specification.
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

    /// Return true if the current specification can be treated as a
    /// fully qualified version, either calendar or semantic.
    pub fn is_fully_qualified(&self) -> bool {
        matches!(self, Self::Version(_))
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
    /// Furthermore, the `Range` and `Requirement` variants will return a
    /// "latest" alias, as they are not resolved or valid versions.
    pub fn to_resolved_spec(&self) -> VersionSpec {
        match self {
            Self::Canary => VersionSpec::Canary,
            Self::Alias(alias) => VersionSpec::Alias(alias.to_owned()),
            Self::Version(version) => VersionSpec::Version(version.to_owned()),
            _ => VersionSpec::default(),
        }
    }

    /// Convert the current unresolved specification to a partial string, where
    /// minor and patch versions are omitted if not defined, and the comparator
    /// operator is also omitted. For example, "~1.2" would simply print "1.2".
    ///
    /// Furthermore, `Canary` will return "canary", `ReqAny` will return "latest",
    /// and aliases will return as-is.
    pub fn to_partial_string(&self) -> String {
        fn from_parts(major: u32, minor: Option<u32>, patch: Option<u32>, pre: &str) -> String {
            let mut version = format!("{major}");

            minor.inspect(|m| {
                version.push_str(&format!(".{m}"));
            });

            patch.inspect(|p| {
                version.push_str(&format!(".{p}"));
            });

            if !pre.is_empty() {
                version.push('-');
                version.push_str(pre);
            }

            version
        }

        match self {
            UnresolvedVersionSpec::Canary => "canary".into(),
            UnresolvedVersionSpec::Alias(alias) => alias.to_string(),
            UnresolvedVersionSpec::Range(_) => "latest".into(),
            UnresolvedVersionSpec::Requirement(req) => from_parts(
                req.major.unwrap_or_default(),
                req.minor,
                req.patch,
                req.prerelease.as_deref().unwrap_or_default(),
            ),
            UnresolvedVersionSpec::Version(ver) => {
                let version = from_parts(
                    ver.major,
                    Some(ver.minor),
                    Some(ver.patch),
                    ver.prerelease.as_deref().unwrap_or_default(),
                );

                match &ver.scope {
                    Some(scope) => format!("{scope}-{version}"),
                    None => version,
                }
            }
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

        // The grammar is the authority on what a version looks like, so try the
        // most specific shape first, and treat an alias as the residual: it is
        // whatever is not structurally a version, requirement, or range

        if let Ok(version) = Version::parse(value) {
            return Ok(Self::Version(version));
        }

        // Kept for the error below, as a failed requirement is the most
        // useful diagnostic for an input that was meant to be a version
        let error = match Requirement::parse(value) {
            Ok(req) => return Ok(Self::Requirement(req)),
            Err(error) => error,
        };

        if let Ok(range) = Range::parse(value) {
            return Ok(Self::Range(range));
        }

        match parse_alias(value) {
            Ok(alias) => Ok(Self::Alias(alias)),
            Err(_) => Err(error),
        }
    }
}

impl TryFrom<String> for UnresolvedVersionSpec {
    type Error = SpecError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl From<UnresolvedVersionSpec> for String {
    fn from(value: UnresolvedVersionSpec) -> Self {
        value.to_string()
    }
}

impl Display for UnresolvedVersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canary => write!(f, "canary"),
            Self::Alias(alias) => write!(f, "{alias}"),
            Self::Range(range) => write!(f, "{range}"),
            Self::Requirement(req) => write!(f, "{req}"),
            Self::Version(version) => write!(f, "{version}"),
        }
    }
}

impl PartialEq<VersionSpec> for UnresolvedVersionSpec {
    fn eq(&self, other: &VersionSpec) -> bool {
        match (self, other) {
            (Self::Canary, VersionSpec::Canary) => true,
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
            (Self::Version(l), Self::Version(r)) => l.cmp(r),

            // Use human sorting for requirements/ranges
            _ => compare(&self.to_string(), &other.to_string()),
        }
    }
}
