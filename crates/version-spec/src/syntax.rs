use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Version {
    pub scope: Option<String>,
    pub major: u64, // year
    pub minor: u64, // month
    pub micro: u64, // day
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(scope) = &self.scope {
            write!(f, "{scope}-")?;
        }

        write!(f, "{}.{}.{}", self.major, self.minor, self.micro)?;

        if let Some(pre) = &self.prerelease {
            write!(f, "-{pre}")?;
        }

        if let Some(build) = &self.build {
            write!(f, "+{build}")?;
        }

        Ok(())
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Requirement {
    pub op: Op,
    pub scope: Option<String>,
    pub major: Option<u64>, // year
    pub minor: Option<u64>, // month
    pub micro: Option<u64>, // day
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

impl Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.op)?;

        if let Some(scope) = &self.scope {
            write!(f, "{scope}-")?;
        }

        if let Some(major) = &self.major {
            write!(f, "{major}")?;

            if let Some(minor) = &self.minor {
                write!(f, ".{minor}")?;

                if let Some(micro) = &self.micro {
                    write!(f, ".{micro}")?;
                } else if self.op == Op::Wildcard {
                    f.write_str(".*")?;
                }
            } else if self.op == Op::Wildcard {
                f.write_str(".*")?;
            }
        } else if self.op == Op::Wildcard {
            f.write_str(".*")?;
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Range {
    pub clauses: Vec<Clause>,
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
