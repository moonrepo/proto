mod resolved_spec;
mod shared_string;
mod spec_error;
mod syntax_parser;
mod syntax_range;
mod syntax_requirement;
mod syntax_traits;
mod syntax_version;
mod unresolved_spec;

pub use resolved_spec::*;
pub use shared_string::*;
pub use spec_error::*;
#[doc(hidden)]
pub use syntax_parser::*;
pub use syntax_range::*;
pub use syntax_requirement::*;
pub use syntax_traits::*;
pub use syntax_version::*;
pub use unresolved_spec::*;

use regex::Regex;
use std::sync::OnceLock;

/// Returns true if the provided value looks like a calendar version string,
/// instead of a semantic version string. This is used to determine which parser to use.
pub fn is_calver_like(value: &str) -> bool {
    static CALVER_REGEX: OnceLock<Regex> = OnceLock::new();

    CALVER_REGEX
        .get_or_init(|| {
            Regex::new(
                r"-?(v|V)?([0-9]{2,4})-((0?[1-9]{1})|10|11|12)(-(0?[1-9]{1}|[1-3]{1}[0-9]{1}))?-?",
            )
            .unwrap()
        })
        .is_match(value)
}

/// Normalizes a version as some tools publish it into one that the strict
/// parsers accept, for example a PEP 440 version like `3.15.0rc2`.
///
/// - A pre-release attached to the version, which starts with a letter, is
///   separated with a `-`, for example `3.13.0t` becomes `3.13.0-t`. When its
///   leading label is followed by a number, the number is separated with a
///   `.`, so that it orders numerically, for example `3.15.0rc2` becomes
///   `3.15.0-rc.2`, and `1.0.0a1.dev2` becomes `1.0.0-a.1.dev2`. The
///   pre-release is otherwise kept as written, and may contain letters,
///   numbers, `-`, `_`, and `.`.
/// - When such a pre-release exists, the version is filled to 3 parts, as a
///   pre-release requires a full version, for example `3.0rc1` becomes `3.0.0-rc.1`.
/// - Leading zeros are removed from the version parts, for example
///   `2021.01.07` becomes `2021.1.7`.
///
/// Anything else is kept as-is, including a pre-release that already follows
/// a `-`, a leading `v`, and build metadata, so a value that already parses
/// keeps its meaning. A value that is not shaped like a version, like a
/// requirement or an alias, is returned unchanged. So is a pre-release whose
/// leading label is followed by a `.` and a number, like `1.0.0rc.1`, as it
/// would normalize the same as `1.0.0rc1`, and could not be converted back.
///
/// As the pre-release may be anything, this doesn't know what it means to the
/// tool, for example a PEP 440 post-release like `1.0.0post1` becomes the
/// pre-release `1.0.0-post.1`, which orders before `1.0.0`.
///
/// To convert a normalized version back into the tool's format, use
/// [`FormatOptions`] with [`attach_prerelease`](FormatOptions::attach_prerelease),
/// and pad widths for any removed zeros.
pub fn normalize_version(value: &str) -> String {
    static VERSION_REGEX: OnceLock<Regex> = OnceLock::new();

    let value = value.trim();
    let regex = VERSION_REGEX.get_or_init(|| {
        Regex::new(
            r"^(?<prefix>[vV]?)(?<parts>[0-9]+(?:\.[0-9]+)*)(?:(?<label>[A-Za-z]+)(?<pre>[0-9A-Za-z_.-]*)(?<build>\+.*)?|(?<rest>[-+].*))?$",
        )
        .unwrap()
    });

    let Some(caps) = regex.captures(value) else {
        return value.to_owned();
    };

    let mut parts = caps["parts"]
        .split('.')
        .map(|part| match part.trim_start_matches('0') {
            "" => "0",
            part => part,
        })
        .collect::<Vec<_>>();

    let mut out = String::from(&caps["prefix"]);

    if let Some(label) = caps.name("label") {
        let pre = &caps["pre"];

        // The reverse removes the `.` between the label and a number,
        // so a pre-release that already has one can't be converted back
        if pre
            .strip_prefix('.')
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        {
            return value.to_owned();
        }

        while parts.len() < 3 {
            parts.push("0");
        }

        out.push_str(&parts.join("."));
        out.push('-');
        out.push_str(label.as_str());

        if pre.starts_with(|c: char| c.is_ascii_digit()) {
            out.push('.');
        }

        out.push_str(pre);

        if let Some(build) = caps.name("build") {
            out.push_str(build.as_str());
        }
    } else {
        out.push_str(&parts.join("."));

        if let Some(rest) = caps.name("rest") {
            out.push_str(rest.as_str());
        }
    }

    out
}

/// Returns true if the provided value looks like a semver version string,
/// instead of a calendar version string. This is used to determine which parser to use.
pub fn is_semver_like(value: &str) -> bool {
    static SEMVER_REGEX: OnceLock<Regex> = OnceLock::new();

    SEMVER_REGEX
        .get_or_init(|| Regex::new(r"-?(v|V)?[0-9]+\.[0-9]+(\.[0-9]+)?-?").unwrap())
        .is_match(value)
}
