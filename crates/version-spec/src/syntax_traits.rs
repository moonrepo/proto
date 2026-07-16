use crate::syntax::{Clause, Op, Range, Requirement, Version};

/// Trait for matching a version against the implementing type.
pub trait MatchesVersion {
    /// Returns true if the provided version satisfies this shape,
    /// following the same rules as the [`semver`] crate.
    fn matches(&self, version: &Version) -> bool;
}

impl MatchesVersion for Version {
    fn matches(&self, version: &Version) -> bool {
        self == version
    }
}

impl MatchesVersion for Requirement {
    fn matches(&self, version: &Version) -> bool {
        self.matches_op(version) && (version.prerelease.is_none() || self.matches_pre(version))
    }
}

impl MatchesVersion for Clause {
    fn matches(&self, version: &Version) -> bool {
        match self {
            Clause::All(reqs) => {
                reqs.iter().all(|req| req.matches_op(version))
                    && (version.prerelease.is_none()
                        || reqs.iter().any(|req| req.matches_pre(version)))
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

impl MatchesVersion for Range {
    fn matches(&self, version: &Version) -> bool {
        if self.clauses.is_empty() {
            return version.prerelease.is_none();
        }

        self.clauses.iter().any(|clause| clause.matches(version))
    }
}
