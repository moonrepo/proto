use crate::is_calver_like;
use crate::spec_error::SpecError;
use crate::syntax_parser::{calendar_year, parse_calver, parse_semver};
use crate::syntax_requirement::{Op, Requirement};
use crate::syntax_traits::{FormatOptions, FormatsVersion};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{self, Display};
use std::str::FromStr;

/// The kind of version, either calendar or semantic.
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum VersionKind {
    /// A calendar version, typically in the form of `YYYY-MM-DD` or `YYYY-MM`.
    Calendar,

    /// A semantic version, typically in the form of `MAJOR.MINOR.PATCH`.
    #[default]
    Semantic,
}

/// A version in either calendar or semantic format, with support for
/// scopes, pre-releases, and build metadata.
///
/// Versions are grouped by scope, with unscoped versions first, and then
/// ordered by precedence per the semver spec: the major, minor, and patch
/// numbers, then the pre-release. As the kind and build metadata do not
/// affect precedence, they are only used as tiebreakers.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    /// The kind of version.
    pub kind: VersionKind,

    /// An optional scope prefix, for example the "vendor" in `vendor-1.2.3`.
    /// Does not include the trailing `-`.
    pub scope: Option<CompactString>,

    /// The major version number, or the year for calendar versions.
    pub major: u32,

    /// The minor version number, or the month for calendar versions.
    pub minor: u32,

    /// The patch version number, or the day for calendar versions,
    /// in which a day of 0 means it was not defined.
    pub patch: u32,

    /// Optional pre-release identifier, for example the "alpha.1"
    /// in `1.2.3-alpha.1`. Does not include the leading `-`.
    pub prerelease: Option<CompactString>,

    /// Optional build metadata, for example the "build.5" in `1.2.3+build.5`.
    /// Does not include the leading `+`.
    pub build: Option<CompactString>,
}

impl Version {
    /// Creates a semantic version from the provided major, minor,
    /// and patch numbers.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self::semantic(major, minor, patch)
    }

    /// Creates a calendar version from the provided year, month, and day.
    /// Short years are expanded from the year 2000, while months and days
    /// are clamped to valid ranges.
    pub fn calendar(year: u32, month: u32, day: u32) -> Self {
        Self {
            kind: VersionKind::Calendar,
            major: calendar_year(year),
            minor: month.clamp(1, 12),
            patch: day.clamp(1, 31),
            ..Default::default()
        }
    }

    /// Creates a semantic version from the provided major, minor,
    /// and patch numbers.
    pub fn semantic(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            kind: VersionKind::Semantic,
            major,
            minor,
            patch,
            ..Default::default()
        }
    }

    /// Parses the provided value into a version.
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, SpecError> {
        let value = value.as_ref();

        // The calendar check may false-positive on inner version parts,
        // like the "20.3" in "10.20.30", so fall back to semantic
        if is_calver_like(value) {
            parse_calver(value).or_else(|_| parse_semver(value))
        } else {
            parse_semver(value)
        }
        .map_err(|error| SpecError::FailedVersionParse {
            error: Box::new(error),
        })
    }

    /// Return true if the version is a calendar version.
    pub fn is_calendar(&self) -> bool {
        self.kind == VersionKind::Calendar
    }

    /// Return true if the version is a semantic version.
    pub fn is_semantic(&self) -> bool {
        self.kind == VersionKind::Semantic
    }

    /// Converts this version into a requirement with the provided operator.
    pub fn to_requirement(&self, op: Op) -> Requirement {
        Requirement {
            kind: self.kind,
            op,
            scope: self.scope.clone(),
            major: Some(self.major),
            minor: Some(self.minor),
            patch: if self.kind == VersionKind::Calendar && self.patch == 0 {
                None
            } else {
                Some(self.patch)
            },
            prerelease: self.prerelease.clone(),
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_formatted_string(&match self.kind {
                VersionKind::Calendar => FormatOptions {
                    include_patch: self.patch > 0,
                    ..FormatOptions::calendar()
                },
                VersionKind::Semantic => FormatOptions::semantic(),
            })
        )
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.scope
            .cmp(&other.scope)
            .then_with(|| self.major.cmp(&other.major))
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| {
                compare_prerelease(self.prerelease.as_deref(), other.prerelease.as_deref())
            })
            .then_with(|| self.kind.cmp(&other.kind))
            .then_with(|| compare_build(self.build.as_deref(), other.build.as_deref()))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Version> for String {
    fn from(value: Version) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Version {
    type Error = SpecError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl FromStr for Version {
    type Err = SpecError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for Version {
    fn schema_name() -> Option<String> {
        Some("Version".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}

// A version without a pre-release compares greater than one with a
// pre-release. Identifiers are compared per the semver spec: numerically
// for digit-only identifiers, lexically otherwise, with numeric identifiers
// having lower precedence, and a larger set having a higher precedence
pub(crate) fn compare_prerelease(lhs: Option<&str>, rhs: Option<&str>) -> Ordering {
    let (lhs, rhs) = match (lhs, rhs) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Greater,
        (Some(_), None) => return Ordering::Less,
        (Some(lhs), Some(rhs)) => (lhs, rhs),
    };

    let mut rhs_parts = rhs.split('.');

    for lhs_part in lhs.split('.') {
        let Some(rhs_part) = rhs_parts.next() else {
            return Ordering::Greater;
        };

        let is_digits = |value: &str| value.bytes().all(|byte| byte.is_ascii_digit());

        let ordering = match (is_digits(lhs_part), is_digits(rhs_part)) {
            // Respect numeric ordering, for example 99 < 100
            (true, true) => lhs_part
                .len()
                .cmp(&rhs_part.len())
                .then_with(|| lhs_part.cmp(rhs_part)),
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => lhs_part.cmp(rhs_part),
        };

        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    if rhs_parts.next().is_none() {
        Ordering::Equal
    } else {
        Ordering::Less
    }
}

// No build metadata compares less than any build metadata. Identifiers
// are compared like pre-releases, except leading zeros on digit-only
// identifiers are also ordered, for example "0" < "00" < "1" < "01" < "2"
fn compare_build(lhs: Option<&str>, rhs: Option<&str>) -> Ordering {
    let (lhs, rhs) = match (lhs, rhs) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Less,
        (Some(_), None) => return Ordering::Greater,
        (Some(lhs), Some(rhs)) => (lhs, rhs),
    };

    let mut rhs_parts = rhs.split('.');

    for lhs_part in lhs.split('.') {
        let Some(rhs_part) = rhs_parts.next() else {
            return Ordering::Greater;
        };

        let is_digits = |value: &str| value.bytes().all(|byte| byte.is_ascii_digit());

        let ordering = match (is_digits(lhs_part), is_digits(rhs_part)) {
            (true, true) => {
                let lhs_trimmed = lhs_part.trim_start_matches('0');
                let rhs_trimmed = rhs_part.trim_start_matches('0');

                lhs_trimmed
                    .len()
                    .cmp(&rhs_trimmed.len())
                    .then_with(|| lhs_trimmed.cmp(rhs_trimmed))
                    .then_with(|| lhs_part.len().cmp(&rhs_part.len()))
            }
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => lhs_part.cmp(rhs_part),
        };

        if ordering != Ordering::Equal {
            return ordering;
        }
    }

    if rhs_parts.next().is_none() {
        Ordering::Equal
    } else {
        Ordering::Less
    }
}
