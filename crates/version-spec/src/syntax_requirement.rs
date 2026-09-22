use crate::is_calver_like;
use crate::spec_error::SpecError;
use crate::syntax_parser::{parse_calver_req, parse_semver_req};
use crate::syntax_traits::{FormatOptions, FormatsVersion};
use crate::syntax_version::{Version, VersionKind, compare_prerelease};
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{self, Display};
use std::str::FromStr;

/// The comparison operator of a requirement.
///
/// Operators are ordered by the versions they match against the same
/// version, from the lowest to the highest, for example `<1.2.3`, `=1.2.3`,
/// `~1.2.3`, `^1.2.3`, and then `>1.2.3`.
#[derive(Copy, Clone, Debug, Default, Eq, Hash, PartialEq)]
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
            Self::Exact => "=",
            Self::Greater => ">",
            Self::GreaterEq => ">=",
            Self::Less => "<",
            Self::LessEq => "<=",
            Self::Tilde => "~",
            Self::Caret => "^",
            Self::Wildcard => "",
        })
    }
}

impl Op {
    // Operators that match from lower versions rank first, and among
    // those that start matching on the same version, the operators that
    // stop matching on lower versions rank first
    fn rank(self) -> u8 {
        match self {
            Self::Less => 0,
            Self::LessEq => 1,
            Self::Exact => 2,
            Self::Wildcard => 3,
            Self::Tilde => 4,
            Self::Caret => 5,
            Self::GreaterEq => 6,
            Self::Greater => 7,
        }
    }
}

impl Ord for Op {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl PartialOrd for Op {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A version requirement composed of a comparison operator and a full
/// or partial version to match against. Build metadata is accepted
/// when parsing, but is otherwise ignored.
///
/// Requirements are ordered like the version they reference, in which an
/// omitted part orders before any number, for example `1` before `1.0`,
/// and then by operator (see [`Op`]). As the kind does not affect
/// matching, it is only used as a tiebreaker.
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
        self.scope
            .cmp(&other.scope)
            .then_with(|| self.major.cmp(&other.major))
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| {
                compare_prerelease(self.prerelease.as_deref(), other.prerelease.as_deref())
            })
            .then_with(|| self.op.cmp(&other.op))
            .then_with(|| self.kind.cmp(&other.kind))
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
