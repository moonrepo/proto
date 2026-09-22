use crate::is_calver_like;
use crate::spec_error::SpecError;
use crate::syntax_parser::{parse_calver_range, parse_semver_range};
use crate::syntax_requirement::{Op, Requirement};
use crate::syntax_version::Version;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{self, Display};
use std::str::FromStr;

/// A single clause within a version range.
///
/// Clauses are ordered by their requirements, from the lowest to the
/// highest, regardless of the order they were written in, in which
/// a bounded range is treated as `>=lower && <=upper`. When those are
/// equal, the kind of clause, and then the written order, are used
/// as tiebreakers.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
            Self::All(reqs) => {
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
            Self::Between(ver1, ver2) => {
                if ver1.scope == ver2.scope {
                    ver1.scope.as_deref()
                } else {
                    None
                }
            }
            Self::Only(req) => req.scope.as_deref(),
        }
    }

    /// Set the scope on either the current requirement(s) or version(s).
    pub fn set_scope(&mut self, scope: impl AsRef<str>) {
        let scope = Some(scope.as_ref().into());

        match self {
            Self::All(reqs) => {
                for req in reqs {
                    req.scope = scope.clone();
                }
            }
            Self::Between(ver1, ver2) => {
                ver1.scope = scope.clone();
                ver2.scope = scope;
            }
            Self::Only(req) => {
                req.scope = scope;
            }
        }
    }

    // A bounded range is converted into requirements, which drops
    // the build metadata, so it must also be compared separately
    fn to_sorted_requirements(&self) -> Vec<Cow<'_, Requirement>> {
        let mut reqs = match self {
            Self::All(reqs) => reqs.iter().map(Cow::Borrowed).collect(),
            Self::Between(lower, upper) => vec![
                Cow::Owned(lower.to_requirement(Op::GreaterEq)),
                Cow::Owned(upper.to_requirement(Op::LessEq)),
            ],
            Self::Only(req) => vec![Cow::Borrowed(req)],
        };

        reqs.sort();
        reqs
    }

    fn rank(&self) -> u8 {
        match self {
            Self::All(_) => 0,
            Self::Between(_, _) => 1,
            Self::Only(_) => 2,
        }
    }
}

impl Ord for Clause {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_sorted_requirements()
            .cmp(&other.to_sorted_requirements())
            .then_with(|| self.rank().cmp(&other.rank()))
            .then_with(|| match (self, other) {
                (Self::All(lhs), Self::All(rhs)) => lhs.cmp(rhs),
                (Self::Between(lhs_lower, lhs_upper), Self::Between(rhs_lower, rhs_upper)) => {
                    lhs_lower
                        .cmp(rhs_lower)
                        .then_with(|| lhs_upper.cmp(rhs_upper))
                }
                (Self::Only(lhs), Self::Only(rhs)) => lhs.cmp(rhs),
                _ => Ordering::Equal,
            })
    }
}

impl PartialOrd for Clause {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::All(reqs) => {
                for (i, req) in reqs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" && ")?;
                    }

                    write!(f, "{req}")?;
                }

                Ok(())
            }
            Self::Between(ver1, ver2) => write!(f, "{ver1} - {ver2}"),
            Self::Only(req) => write!(f, "{req}"),
        }
    }
}

/// A version range composed of clauses, in which any clause may match,
/// for example `^1 || 2.3.4 - 3.0.0 || >=4, <5`.
///
/// Ranges are ordered by their clauses (see [`Clause`]), from the lowest
/// to the highest, regardless of the order they were written in, in which
/// an empty range orders first. When those are equal, the written order
/// is used as a tiebreaker.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

impl Ord for Range {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut lhs = self.clauses.iter().collect::<Vec<_>>();
        let mut rhs = other.clauses.iter().collect::<Vec<_>>();

        lhs.sort();
        rhs.sort();

        lhs.cmp(&rhs).then_with(|| self.clauses.cmp(&other.clauses))
    }
}

impl PartialOrd for Range {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
