use crate::spec_error::SpecError;
use crate::syntax_parser::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{self, Display};

#[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum VersionKind {
    Calendar,
    #[default]
    Semantic,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    pub kind: VersionKind,
    pub scope: Option<String>,
    pub major: u64, // year
    pub minor: u64, // month
    pub micro: u64, // day
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

impl Version {
    pub fn calendar(year: u64, month: u64, day: u64) -> Self {
        Self {
            kind: VersionKind::Calendar,
            major: calendar_year(year),
            minor: month.clamp(1, 12),
            micro: day.clamp(1, 31),
            ..Default::default()
        }
    }

    pub fn semantic(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            kind: VersionKind::Semantic,
            major,
            minor,
            micro: patch,
            ..Default::default()
        }
    }

    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, SpecError> {
        let value = value.as_ref().trim();

        // Attempt semantic first, as calendar would consume dotted triples
        // with small numbers, like "1.2.3", as the year 2001. Dashed and
        // partial dotted calendars never match a full semantic version
        if let Ok(semantic) = parse_semver(value) {
            return Ok(semantic);
        }

        // Then attempt calendar
        parse_calver(value).map_err(|error| SpecError::FailedVersionParse { error })
    }

    // Convert into a requirement bound for range matching. A calendar
    // version without a day (0) becomes a month-granular bound
    fn to_requirement(&self, op: Op) -> Requirement {
        Requirement {
            kind: self.kind,
            op,
            scope: self.scope.clone(),
            major: Some(self.major),
            minor: Some(self.minor),
            micro: if self.kind == VersionKind::Calendar && self.micro == 0 {
                None
            } else {
                Some(self.micro)
            },
            prerelease: self.prerelease.clone(),
            build: None,
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(scope) = &self.scope {
            write!(f, "{scope}-")?;
        }

        let sep = match self.kind {
            VersionKind::Calendar => "-",
            VersionKind::Semantic => ".",
        };

        write!(f, "{}{sep}{}", self.major, self.minor)?;

        // A calendar day of 0 means it was not defined
        if self.kind != VersionKind::Calendar || self.micro > 0 {
            write!(f, "{sep}{}", self.micro)?;
        }

        if let Some(pre) = &self.prerelease {
            write!(f, "-{pre}")?;
        }

        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
        }

        Ok(())
    }
}

// Group by kind and scope first, then compare version numbers,
// pre-releases, and build metadata per the semver spec
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| self.scope.cmp(&other.scope))
            .then_with(|| self.major.cmp(&other.major))
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.micro.cmp(&other.micro))
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

#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Op {
    #[default]
    Exact,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Tilde,
    Caret,
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Requirement {
    pub kind: VersionKind,
    pub op: Op,
    pub scope: Option<String>,
    pub major: Option<u64>, // year
    pub minor: Option<u64>, // month
    pub micro: Option<u64>, // day
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

impl Requirement {
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, SpecError> {
        let value = value.as_ref().trim();

        // Attempt semantic first, as calendar would consume partial dotted
        // versions with small numbers, like "1.2", as the year 2001
        if let Ok(semantic) = parse_semver_req(value) {
            return Ok(semantic);
        }

        parse_calver_req(value)
            .map_err(|error| SpecError::FailedVersionRequirementParse { error })
    }

    // Return true if the provided version satisfies this requirement,
    // following the same rules as the semver crate. A version with a
    // pre-release only matches when the requirement also has a
    // pre-release on the same version numbers
    pub fn matches(&self, version: &Version) -> bool {
        self.matches_op(version) && (version.prerelease.is_none() || self.matches_pre(version))
    }

    fn matches_op(&self, version: &Version) -> bool {
        // A scoped requirement only matches the same scope,
        // while an unscoped requirement matches any scope
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

    fn matches_exact(&self, version: &Version) -> bool {
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

        if let Some(micro) = self.micro {
            if version.micro != micro {
                return false;
            }
        }

        self.prerelease == version.prerelease
    }

    fn matches_greater(&self, version: &Version) -> bool {
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

        let Some(micro) = self.micro else {
            return false;
        };

        if version.micro != micro {
            return version.micro > micro;
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            == Ordering::Greater
    }

    fn matches_less(&self, version: &Version) -> bool {
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

        let Some(micro) = self.micro else {
            return false;
        };

        if version.micro != micro {
            return version.micro < micro;
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            == Ordering::Less
    }

    fn matches_tilde(&self, version: &Version) -> bool {
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

        if let Some(micro) = self.micro {
            if version.micro != micro {
                return version.micro > micro;
            }
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            != Ordering::Less
    }

    fn matches_caret(&self, version: &Version) -> bool {
        let Some(major) = self.major else {
            return true;
        };

        if version.major != major {
            return false;
        }

        let Some(minor) = self.minor else {
            return true;
        };

        let Some(micro) = self.micro else {
            return if major > 0 {
                version.minor >= minor
            } else {
                version.minor == minor
            };
        };

        if major > 0 {
            if version.minor != minor {
                return version.minor > minor;
            } else if version.micro != micro {
                return version.micro > micro;
            }
        } else if minor > 0 {
            if version.minor != minor {
                return false;
            } else if version.micro != micro {
                return version.micro > micro;
            }
        } else if version.minor != minor || version.micro != micro {
            return false;
        }

        compare_prerelease(version.prerelease.as_deref(), self.prerelease.as_deref())
            != Ordering::Less
    }

    fn matches_pre(&self, version: &Version) -> bool {
        self.prerelease.is_some()
            && self.major == Some(version.major)
            && self.minor == Some(version.minor)
            && self.micro == Some(version.micro)
    }
}

impl Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.op)?;

        if let Some(scope) = &self.scope {
            write!(f, "{scope}-")?;
        }

        let sep = match self.kind {
            VersionKind::Calendar => "-",
            VersionKind::Semantic => ".",
        };

        if let Some(major) = &self.major {
            write!(f, "{major}")?;

            if let Some(minor) = &self.minor {
                write!(f, "{sep}{minor}")?;

                if let Some(micro) = &self.micro {
                    write!(f, "{sep}{micro}")?;
                } else if self.op == Op::Wildcard {
                    write!(f, "{sep}*")?;
                }
            } else if self.op == Op::Wildcard {
                write!(f, "{sep}*")?;
            }
        } else if self.op == Op::Wildcard {
            f.write_str("*")?;
        }

        if let Some(pre) = &self.prerelease {
            write!(f, "-{pre}")?;
        }

        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
        }

        Ok(())
    }
}

// Same ordering as versions, with wildcard (none) parts ordered
// first, and the operator as the final tiebreaker
impl Ord for Requirement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.kind
            .cmp(&other.kind)
            .then_with(|| self.scope.cmp(&other.scope))
            .then_with(|| self.major.cmp(&other.major))
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.micro.cmp(&other.micro))
            .then_with(|| {
                compare_prerelease(self.prerelease.as_deref(), other.prerelease.as_deref())
            })
            .then_with(|| compare_build(self.build.as_deref(), other.build.as_deref()))
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

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Clause {
    And(Requirement, Requirement),
    Between(Version, Version),
    Only(Requirement),
}

impl Clause {
    // Return true if the provided version satisfies this clause. Pre-release
    // compatibility only needs to be satisfied by one of the requirements
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Clause::And(left, right) => {
                left.matches_op(version)
                    && right.matches_op(version)
                    && (version.prerelease.is_none()
                        || left.matches_pre(version)
                        || right.matches_pre(version))
            }

            // Bounded ranges are inclusive on both ends
            Clause::Between(lower, upper) => {
                let lower = lower.to_requirement(Op::GreaterEq);
                let upper = upper.to_requirement(Op::LessEq);

                lower.matches_op(version)
                    && upper.matches_op(version)
                    && (version.prerelease.is_none()
                        || lower.matches_pre(version)
                        || upper.matches_pre(version))
            }

            Clause::Only(req) => req.matches(version),
        }
    }
}

impl Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Clause::And(req1, req2) => write!(f, "{req1} && {req2}"),
            Clause::Between(ver1, ver2) => write!(f, "{ver1} - {ver2}"),
            Clause::Only(req) => write!(f, "{req}"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Range {
    pub clauses: Vec<Clause>,
}

impl Range {
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, SpecError> {
        let value = value.as_ref().trim();

        // Attempt semantic first, as calendar would consume partial dotted
        // versions with small numbers, like "1.2", as the year 2001
        if let Ok(semantic) = parse_semver_range(value) {
            return Ok(semantic);
        }

        parse_calver_range(value).map_err(|error| SpecError::FailedVersionRangeParse { error })
    }

    // Return true if the provided version satisfies any clause within
    // this range. An empty (wildcard) range matches all versions,
    // except pre-releases
    pub fn matches(&self, version: &Version) -> bool {
        if self.clauses.is_empty() {
            return version.prerelease.is_none();
        }

        self.clauses.iter().any(|clause| clause.matches(version))
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
