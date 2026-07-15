use crate::spec_error::SpecError;
use crate::syntax_parser::*;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
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

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Clause {
    And(Requirement, Requirement),
    Between(Version, Version),
    Only(Requirement),
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
