use crate::is_calver_like;
use crate::spec_error::SpecError;
use crate::syntax_parser::*;
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
        self.kind
            .cmp(&other.kind)
            .then_with(|| self.scope.cmp(&other.scope))
            .then_with(|| self.major.cmp(&other.major))
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| {
                compare_prerelease(self.prerelease.as_deref(), other.prerelease.as_deref())
            })
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

/// The comparison operator of a requirement.
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Op {
    /// An exact match (`=` or `==`).
    Exact,

    /// A greater than match (`>`).
    Greater,

    /// A greater than or equal match (`>=`).
    GreaterEq,

    /// A less than match (`<`).
    Less,

    /// A less than or equal match (`<=`).
    LessEq,

    /// A patch-level match (`~`).
    /// This is the default operator when one is not defined.
    #[default]
    Tilde,

    /// A compatible, up to the next major version, match (`^`).
    Caret,

    /// Matches any version (`*`, `x`, or `X`).
    Wildcard,
}

impl Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Op::Exact => "=",
            Op::Greater => ">",
            Op::GreaterEq => ">=",
            Op::Less => "<",
            Op::LessEq => "<=",
            Op::Tilde => "~",
            Op::Caret => "^",
            Op::Wildcard => "",
        })
    }
}

/// A version requirement composed of a comparison operator and a full
/// or partial version to match against. Build metadata is accepted
/// when parsing, but is otherwise ignored.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Requirement {
    /// The kind of version.
    pub kind: VersionKind,

    /// The comparison operator.
    pub op: Op,

    /// An optional scope prefix, for example the "vendor" in `vendor-1.2`.
    pub scope: Option<CompactString>,

    /// The major version number, or the year for calendar versions.
    /// A `None` is either an omitted part or a wildcard.
    pub major: Option<u32>,

    /// The minor version number, or the month for calendar versions.
    /// A `None` is either an omitted part or a wildcard.
    pub minor: Option<u32>,

    /// The patch version number, or the day for calendar versions.
    /// A `None` is either an omitted part or a wildcard.
    pub patch: Option<u32>,

    /// Optional pre-release identifier, for example the "alpha.1"
    /// in `>=1.2.3-alpha.1`.
    pub prerelease: Option<CompactString>,
}

impl Requirement {
    /// Parses the provided value into a requirement.
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, SpecError> {
        let value = value.as_ref();

        // The calendar check may false-positive on inner version parts,
        // like the "16-1" in "node-16-1.2", so fall back to semantic
        if is_calver_like(value) {
            parse_calver_req(value).or_else(|_| parse_semver_req(value))
        } else {
            parse_semver_req(value)
        }
        .map_err(|error| SpecError::FailedVersionRequirementParse {
            error: Box::new(error),
        })
    }

    /// Returns true if the provided version satisfies the requirement's
    /// operator, without checking pre-release compatibility. A scoped
    /// requirement only matches versions with the same scope, while an
    /// unscoped requirement matches any scope.
    pub fn matches_op(&self, version: &Version) -> bool {
        if self.scope.is_some() && self.scope != version.scope {
            return false;
        }

        match self.op {
            Op::Exact | Op::Wildcard => self.matches_exact(version),
            Op::Greater => self.matches_greater(version),
            Op::GreaterEq => self.matches_exact(version) || self.matches_greater(version),
            Op::Less => self.matches_less(version),
            Op::LessEq => self.matches_exact(version) || self.matches_less(version),
            Op::Tilde => self.matches_tilde(version),
            Op::Caret => self.matches_caret(version),
        }
    }

    /// Returns true if the provided version exactly matches all defined
    /// parts of this requirement, including the pre-release. Omitted
    /// parts match any value.
    pub fn matches_exact(&self, version: &Version) -> bool {
        if let Some(major) = self.major {
            if version.major != major {
                return false;
            }
        }

        if let Some(minor) = self.minor {
            if version.minor != minor {
                return false;
            }
        }

        if let Some(micro) = self.patch {
            if version.patch != micro {
                return false;
            }
        }

        self.prerelease == version.prerelease
    }

    /// Returns true if the provided version is greater (`>`) than this
    /// requirement. A partial requirement only matches versions beyond
    /// the defined parts, for example `>1` does not match `1.5.0`.
    pub fn matches_greater(&self, version: &Version) -> bool {
        let Some(major) = self.major else {
            return false;
        };

        if version.major != major {
            return version.major > major;
        }

        let Some(minor) = self.minor else {
            return false;
        };

        if version.minor != minor {
            return version.minor > minor;
        }

        let Some(micro) = self.patch else {
            return false;
        };

        if version.patch != micro {
            return version.patch > micro;
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            == Ordering::Greater
    }

    /// Returns true if the provided version is less (`<`) than this requirement.
    /// A partial requirement only matches versions below the defined parts.
    pub fn matches_less(&self, version: &Version) -> bool {
        let Some(major) = self.major else {
            return false;
        };

        if version.major != major {
            return version.major < major;
        }

        let Some(minor) = self.minor else {
            return false;
        };

        if version.minor != minor {
            return version.minor < minor;
        }

        let Some(micro) = self.patch else {
            return false;
        };

        if version.patch != micro {
            return version.patch < micro;
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            == Ordering::Less
    }

    /// Returns true for a patch-level (`~`) match: the defined major and
    /// minor parts must be equal, while the remaining parts may drift higher.
    pub fn matches_tilde(&self, version: &Version) -> bool {
        let Some(major) = self.major else {
            return true;
        };

        if version.major != major {
            return false;
        }

        if let Some(minor) = self.minor {
            if version.minor != minor {
                return false;
            }
        }

        if let Some(micro) = self.patch {
            if version.patch != micro {
                return version.patch > micro;
            }
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            != Ordering::Less
    }

    /// Returns true for a compatible (`^`) match: parts may drift up to
    /// the next major version, or the next minor or patch version when
    /// the major or minor is 0.
    pub fn matches_caret(&self, version: &Version) -> bool {
        let Some(major) = self.major else {
            return true;
        };

        if version.major != major {
            return false;
        }

        let Some(minor) = self.minor else {
            return true;
        };

        let Some(micro) = self.patch else {
            return if major > 0 {
                version.minor >= minor
            } else {
                version.minor == minor
            };
        };

        if major > 0 {
            if version.minor != minor {
                return version.minor > minor;
            } else if version.patch != micro {
                return version.patch > micro;
            }
        } else if minor > 0 {
            if version.minor != minor {
                return false;
            } else if version.patch != micro {
                return version.patch > micro;
            }
        } else if version.minor != minor || version.patch != micro {
            return false;
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            != Ordering::Less
    }

    /// Returns true if this requirement has a pre-release on the same
    /// version numbers as the provided version, allowing a pre-release
    /// version to be matched.
    pub fn matches_pre(&self, version: &Version) -> bool {
        self.prerelease.is_some()
            && self.major == Some(version.major)
            && self.minor == Some(version.minor)
            && self.patch == Some(version.patch)
    }
}

impl Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_formatted_string(&match self.kind {
                VersionKind::Calendar => FormatOptions::calendar(),
                VersionKind::Semantic => FormatOptions::semantic(),
            })
        )
    }
}

impl Ord for Requirement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| self.scope.cmp(&other.scope))
            .then_with(|| self.major.cmp(&other.major))
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| {
                compare_prerelease(self.prerelease.as_deref(), other.prerelease.as_deref())
            })
            .then_with(|| self.op.cmp(&other.op))
    }
}

impl PartialOrd for Requirement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Requirement> for String {
    fn from(value: Requirement) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Requirement {
    type Error = SpecError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl FromStr for Requirement {
    type Err = SpecError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for Requirement {
    fn schema_name() -> Option<String> {
        Some("Requirement".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}

/// A single clause within a version range.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Clause {
    /// A list of requirements that must all match, for example `>=1.2 && <2`.
    All(Vec<Requirement>),

    /// A bounded range between two fully qualified versions, inclusive
    /// on both ends, for example `1.2.3 - 2.3.4`. The versions are boxed
    /// to keep the size of this enum down.
    Between(Box<Version>, Box<Version>),

    /// A single requirement.
    Only(Requirement),
}

impl Clause {
    /// Returns the version scope if available. A clause with multiple requirements
    /// only has a scope if all scoped requirements share the same scope, while
    /// requirements without a scope are ignored.
    pub fn get_scope(&self) -> Option<&str> {
        match self {
            Clause::All(reqs) => {
                let mut scope = None;

                for req in reqs {
                    if let Some(req_scope) = req.scope.as_deref() {
                        if scope.is_none() {
                            scope = Some(req_scope);
                        } else if scope != Some(req_scope) {
                            return None;
                        }
                    }
                }

                scope
            }
            Clause::Between(ver1, ver2) => {
                if ver1.scope == ver2.scope {
                    ver1.scope.as_deref()
                } else {
                    None
                }
            }
            Clause::Only(req) => req.scope.as_deref(),
        }
    }

    /// Set the scope on either the current requirement(s) or version(s).
    pub fn set_scope(&mut self, scope: impl AsRef<str>) {
        let scope = Some(scope.as_ref().into());

        match self {
            Clause::All(reqs) => {
                for req in reqs {
                    req.scope = scope.clone();
                }
            }
            Clause::Between(ver1, ver2) => {
                ver1.scope = scope.clone();
                ver2.scope = scope;
            }
            Clause::Only(req) => {
                req.scope = scope;
            }
        }
    }
}

impl Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Clause::All(reqs) => {
                for (i, req) in reqs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" && ")?;
                    }

                    write!(f, "{req}")?;
                }

                Ok(())
            }
            Clause::Between(ver1, ver2) => write!(f, "{ver1} - {ver2}"),
            Clause::Only(req) => write!(f, "{req}"),
        }
    }
}

/// A version range composed of clauses, in which any clause may match,
/// for example `^1 || 2.3.4 - 3.0.0 || >=4, <5`.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Range {
    /// The list of clauses. An empty list is a wildcard match.
    pub clauses: Vec<Clause>,
}

impl Range {
    /// Parses the provided value into a range, attempting the semantic
    /// format first, and the calendar format second. A leading `v` or `V`
    /// is ignored.
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, SpecError> {
        let value = value.as_ref();

        // The calendar check may false-positive on inner version parts,
        // like the "20.3" in "10.20.30", so fall back to semantic
        if is_calver_like(value) {
            parse_calver_range(value).or_else(|_| parse_semver_range(value))
        } else {
            parse_semver_range(value)
        }
        .map_err(|error| SpecError::FailedVersionRangeParse {
            error: Box::new(error),
        })
    }

    /// Returns the version scope if available. A range with multiple clauses
    /// only has a scope if all scoped clauses share the same scope, while
    /// clauses without a scope are ignored.
    pub fn get_scope(&self) -> Option<&str> {
        let mut scope = None;

        for clause in &self.clauses {
            if let Some(clause_scope) = clause.get_scope() {
                if scope.is_none() {
                    scope = Some(clause_scope);
                } else if scope != Some(clause_scope) {
                    return None;
                }
            }
        }

        scope
    }

    /// Set the scope on all clauses within the range.
    pub fn set_scope(&mut self, scope: impl AsRef<str>) {
        for clause in &mut self.clauses {
            clause.set_scope(scope.as_ref());
        }
    }
}

impl Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.clauses.is_empty() {
            return f.write_str("*");
        }

        for (i, clause) in self.clauses.iter().enumerate() {
            if i > 0 {
                f.write_str(" || ")?;
            }

            write!(f, "{clause}")?;
        }

        Ok(())
    }
}

impl From<Range> for String {
    fn from(value: Range) -> Self {
        value.to_string()
    }
}

impl TryFrom<String> for Range {
    type Error = SpecError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl FromStr for Range {
    type Err = SpecError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for Range {
    fn schema_name() -> Option<String> {
        Some("Range".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}

// A version without a pre-release compares greater than one with a
// pre-release. Identifiers are compared per the semver spec: numerically
// for digit-only identifiers, lexically otherwise, with numeric identifiers
// having lower precedence, and a larger set having a higher precedence
fn compare_prerelease(lhs: Option<&str>, rhs: Option<&str>) -> Ordering {
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
