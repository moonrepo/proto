use crate::syntax::{Clause, Op, Range, Requirement, Version, VersionKind};

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

/// Trait for matching a requirement against the implementing type.
pub trait MatchesRequirement {
    /// Returns true if the provided requirement overlaps this shape, in
    /// which at least one version satisfies both, following the same
    /// rules as [`MatchesVersion`]. For example, `>=1.2.5` overlaps `~1.2`,
    /// as both are satisfied by `1.2.5`.
    fn matches_req(&self, req: &Requirement) -> bool;
}

impl MatchesRequirement for Clause {
    fn matches_req(&self, req: &Requirement) -> bool {
        match self {
            Clause::All(reqs) => {
                has_shared_version(req, reqs, self.get_scope(), |version| self.matches(version))
            }
            Clause::Between(lower, upper) => has_shared_version(
                req,
                &[
                    lower.to_requirement(Op::GreaterEq),
                    upper.to_requirement(Op::LessEq),
                ],
                self.get_scope(),
                |version| self.matches(version),
            ),
            Clause::Only(other) => has_shared_version(
                req,
                std::slice::from_ref(other),
                self.get_scope(),
                |version| self.matches(version),
            ),
        }
    }
}

impl MatchesRequirement for Range {
    fn matches_req(&self, req: &Requirement) -> bool {
        if self.clauses.is_empty() {
            return has_shared_version(req, &[], None, |version| self.matches(version));
        }

        self.clauses.iter().any(|clause| clause.matches_req(req))
    }
}

// Rather than intersecting requirements directly, which would duplicate the
// matching rules, search for a version that satisfies both sides. Within
// a scope, the versions that a requirement matches are contiguous (as a
// part cannot follow a wildcard part), so if any shared version exists,
// the lowest one is the lower bound of one of the requirements involved,
// and only those bounds need to be checked.
//
// For releases, a lower bound is a requirement with its omitted parts
// zeroed, the version after it for an exclusive (`>`) requirement, or
// `0.0.0`. Pre-releases only match when a requirement opts into them on
// the same version numbers, so they are only possible on the provided
// requirement's version numbers, where a lower bound is a requirement's
// pre-release, the pre-release after it, or the lowest possible pre-release
fn has_shared_version(
    req: &Requirement,
    bounds: &[Requirement],
    scope: Option<&str>,
    matches: impl Fn(&Version) -> bool,
) -> bool {
    let scope = req.scope.as_deref().or(scope);
    let all_bounds = || std::iter::once(req).chain(bounds);

    let check = |major: u32, minor: u32, patch: u32, prerelease: Option<&str>| {
        let version = Version {
            kind: req.kind,
            scope: scope.map(Into::into),
            major,
            minor,
            patch,
            prerelease: prerelease.map(Into::into),
            build: None,
        };

        req.matches(&version) && matches(&version)
    };

    // Releases
    if check(0, 0, 0, None) {
        return true;
    }

    for bound in all_bounds() {
        let Some(major) = bound.major else {
            continue;
        };

        if check(
            major,
            bound.minor.unwrap_or(0),
            bound.patch.unwrap_or(0),
            None,
        ) {
            return true;
        }

        if let Some((major, minor, patch)) = next_release(major, bound.minor, bound.patch)
            && check(major, minor, patch, None)
        {
            return true;
        }
    }

    // Pre-releases
    let (Some(major), Some(minor), Some(patch), Some(_)) =
        (req.major, req.minor, req.patch, &req.prerelease)
    else {
        return false;
    };

    // A numeric identifier has the lowest precedence, and fewer
    // identifiers have a lower precedence than more
    if check(major, minor, patch, Some("0")) {
        return true;
    }

    all_bounds().any(|bound| {
        bound.prerelease.as_deref().is_some_and(|pre| {
            check(major, minor, patch, Some(pre))
                || check(major, minor, patch, Some(&format!("{pre}.0")))
        })
    })
}

// Increments the lowest defined part, carrying into
// the higher parts when the maximum is reached
fn next_release(major: u32, minor: Option<u32>, patch: Option<u32>) -> Option<(u32, u32, u32)> {
    match (minor, patch) {
        (Some(minor), Some(patch)) => patch
            .checked_add(1)
            .map(|patch| (major, minor, patch))
            .or_else(|| next_release(major, Some(minor), None)),
        (Some(minor), None) => minor
            .checked_add(1)
            .map(|minor| (major, minor, 0))
            .or_else(|| next_release(major, None, None)),
        _ => major.checked_add(1).map(|major| (major, 0, 0)),
    }
}

/// Options for formatting a version into a string.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Whether to include the comparison operator, and wildcard placeholders,
    /// for requirements.
    pub include_op: bool,
    /// Whether to include the minor version.
    pub include_minor: bool,
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
    /// Create a new `FormatOptions` instance with the specified version kind.
    pub fn new(kind: VersionKind) -> Self {
        match kind {
            VersionKind::Calendar => Self::calendar(),
            VersionKind::Semantic => Self::semantic(),
        }
    }

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
            include_op: true,
            include_minor: true,
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

        if options.include_minor && !(self.kind == VersionKind::Calendar && self.minor == 0) {
            out.push(options.separator);
            pad(&mut out, self.minor, options.pad_minor);

            if options.include_patch && !(self.kind == VersionKind::Calendar && self.patch == 0) {
                out.push(options.separator);
                pad(&mut out, self.patch, options.pad_patch);
            }
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
        let mut out = if options.include_op {
            self.op.to_string()
        } else {
            String::new()
        };

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

            if !options.include_minor {
                // Nothing
            } else if let Some(minor) = &self.minor {
                out.push(options.separator);
                pad(&mut out, minor, options.pad_minor);

                if !options.include_patch {
                    // Nothing
                } else if let Some(patch) = &self.patch {
                    out.push(options.separator);
                    pad(&mut out, patch, options.pad_patch);
                } else if options.include_op && self.op == Op::Wildcard {
                    out.push(options.separator);
                    out.push('*');
                }
            } else if options.include_op && self.op == Op::Wildcard {
                out.push(options.separator);
                out.push('*');
            }
        } else if options.include_op && self.op == Op::Wildcard {
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
