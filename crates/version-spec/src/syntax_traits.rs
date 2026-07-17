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

/// Options for formatting a version into a string.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Whether to include the scope.
    pub include_scope: bool,
    /// Whether to include the patch version.
    pub include_patch: bool,
    /// Whether to include the pre-release information.
    pub include_prerelease: bool,
    /// Whether to include the build metadata.
    pub include_build: bool,
    /// Whether to pad the major version with leading zeros, and if so, how many digits to pad to.
    pub pad_major: Option<u8>,
    /// Whether to pad the minor version with leading zeros, and if so, how many digits to pad to.
    pub pad_minor: Option<u8>,
    /// Whether to pad the patch version with leading zeros, and if so, how many digits to pad to.
    pub pad_patch: Option<u8>,
    /// The separator character to use between version components.
    pub separator: char,
}

impl FormatOptions {
    /// Returns a new `FormatOptions` instance with default settings for calendar versioning.
    pub fn calendar() -> Self {
        Self {
            pad_major: Some(4),
            pad_minor: Some(2),
            pad_patch: Some(2),
            separator: '-',
            ..Default::default()
        }
    }

    /// Returns a new `FormatOptions` instance with default settings for semantic versioning.
    pub fn semantic() -> Self {
        Self::default()
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            include_scope: true,
            include_patch: true,
            include_prerelease: true,
            include_build: true,
            pad_major: None,
            pad_minor: None,
            pad_patch: None,
            separator: '.',
        }
    }
}

/// Trait for formatting a version into a string with custom options.
pub trait FormatsVersion {
    /// Returns a formatted string representation of the version according to the provided options.
    fn to_formatted_string(&self, options: &FormatOptions) -> String;
}

impl FormatsVersion for Version {
    fn to_formatted_string(&self, options: &FormatOptions) -> String {
        let mut out = String::new();

        if options.include_scope
            && let Some(scope) = &self.scope
        {
            out.push_str(scope);
            out.push('-');
        }

        let pad = |out: &mut String, value: u32, width: Option<u8>| {
            if let Some(width) = width {
                let width = width as usize;
                out.push_str(&format!("{value:0>width$}"));
            } else {
                out.push_str(&value.to_string());
            }
        };

        pad(&mut out, self.major, options.pad_major);
        out.push(options.separator);
        pad(&mut out, self.minor, options.pad_minor);

        // A calendar day of 0 means it was not defined
        if options.include_patch {
            out.push(options.separator);
            pad(&mut out, self.patch, options.pad_patch);
        }

        if options.include_prerelease
            && let Some(pre) = &self.prerelease
        {
            out.push('-');
            out.push_str(pre);
        }

        if options.include_build
            && let Some(build) = &self.build
        {
            out.push('+');
            out.push_str(build);
        }

        out
    }
}

impl FormatsVersion for Requirement {
    fn to_formatted_string(&self, options: &FormatOptions) -> String {
        let mut out = self.op.to_string();

        if options.include_scope
            && let Some(scope) = &self.scope
        {
            out.push_str(scope);
            out.push('-');
        }

        let pad = |out: &mut String, value: &u32, width: Option<u8>| {
            if let Some(width) = width {
                let width = width as usize;
                out.push_str(&format!("{value:0>width$}"));
            } else {
                out.push_str(&value.to_string());
            }
        };

        if let Some(major) = &self.major {
            pad(&mut out, major, options.pad_major);

            if let Some(minor) = &self.minor {
                out.push(options.separator);
                pad(&mut out, minor, options.pad_minor);

                if let Some(patch) = &self.patch {
                    out.push(options.separator);
                    pad(&mut out, patch, options.pad_patch);
                } else if self.op == Op::Wildcard {
                    out.push(options.separator);
                    out.push('*');
                }
            } else if self.op == Op::Wildcard {
                out.push(options.separator);
                out.push('*');
            }
        } else if self.op == Op::Wildcard {
            out.push('*');
        }

        if options.include_prerelease
            && let Some(pre) = &self.prerelease
        {
            out.push('-');
            out.push_str(pre);
        }

        out
    }
}
