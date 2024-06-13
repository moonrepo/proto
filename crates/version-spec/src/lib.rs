mod resolved_spec;
mod unresolved_spec;
mod version_types;

pub use resolved_spec::*;
pub use unresolved_spec::*;
pub use version_types::*;

use regex::Regex;
use std::sync::OnceLock;

/// Returns true if the provided value is an alias. An alias is a word that
/// maps to version, for example, "latest" -> "1.2.3".
///
/// Is considered an alias if the string is alpha-numeric, starts with a
/// character, and supports `-`, `_`, `/`, `.`, and `*` characters.
pub fn is_alias_name<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();

    value.chars().enumerate().all(|(i, c)| {
        if i == 0 {
            char::is_ascii_alphabetic(&c)
        } else {
            char::is_ascii_alphanumeric(&c)
                || c == '-'
                || c == '_'
                || c == '/'
                || c == '.'
                || c == '*'
        }
    })
}

/// Returns true if the provided value is a calendar version string.
pub fn is_calver<T: AsRef<str>>(value: T) -> bool {
    get_calver_regex().is_match(value.as_ref())
}

/// Returns true if the provided value is a semantic version string.
pub fn is_semver<T: AsRef<str>>(value: T) -> bool {
    get_semver_regex().is_match(value.as_ref())
}

static CLEAN_REQ_REGEX: OnceLock<Regex> = OnceLock::new();
static CLEAN_WS_REGEX: OnceLock<Regex> = OnceLock::new();
static CLEAN_CAL_REGEX: OnceLock<Regex> = OnceLock::new();
static CLEAN_ZERO_REGEX: OnceLock<Regex> = OnceLock::new();

/// Cleans a potential version string by removing a leading `v` or `V`.
pub fn clean_version_string<T: AsRef<str>>(value: T) -> String {
    let mut version = value.as_ref().trim();

    // Remove a leading "v" or "V" from a version string.
    #[allow(clippy::assigning_clones)]
    if version.starts_with('v') || version.starts_with('V') {
        version = &version[1..];
    }

    version.to_owned()
}

/// Cleans a version requirement string by removing each occurence of `.*`,
/// removing invalid spaces, and replacing calver-like syntax to be semver compatible.
pub fn clean_version_req_string<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref().trim();

    if value.contains("||") {
        return value
            .split("||")
            .map(clean_version_req_string)
            .collect::<Vec<_>>()
            .join(" || ");
    }

    let version = value.replace(".*", "").replace("&&", ",");

    // Remove invalid space after <, <=, >, >=.
    let version = CLEAN_REQ_REGEX
        .get_or_init(|| regex::Regex::new(r"([><]=?)[ ]*v?([0-9])").unwrap())
        .replace_all(&version, "$1$2");

    // Replace spaces with commas
    let version = CLEAN_WS_REGEX
        .get_or_init(|| regex::Regex::new("[, ]+").unwrap())
        .replace_all(&version, ",");

    // Replace calver dashes with periods
    // 2024-2 -> 2024.2
    // 2024-2-alpha -> 2024.2-alpha
    let version = CLEAN_CAL_REGEX
        .get_or_init(|| regex::Regex::new(r"(\d)-(\d)").unwrap())
        .replace_all(&version, "$1.$2");

    // Remove leading zeros from each number part
    let version = CLEAN_ZERO_REGEX
        .get_or_init(|| regex::Regex::new(r"(^|\.)0+(\d)").unwrap())
        .replace_all(&version, "$1$2");

    version.to_string()
}

static CALVER_REGEX: OnceLock<Regex> = OnceLock::new();

/// Get a regex pattern that matches calendar versions (calver).
/// For example: 2024-02-26, 2024-12, 2024-01-alpha, etc.
pub fn get_calver_regex() -> &'static Regex {
    CALVER_REGEX.get_or_init(|| {
        Regex::new(r"^(?<year>[0-9]{1,4})-(?<month>((0?[1-9]{1})|10|11|12))(-(?<day>(0?[1-9]{1}|[1-3]{1}[0-9]{1})))?((_|\.)(?<micro>[0-9]+))?(?<pre>-[a-zA-Z]{1}[-0-9a-zA-Z.]+)?$").unwrap()
    })
}

static SEMVER_REGEX: OnceLock<Regex> = OnceLock::new();

/// Get a regex pattern that matches semantic versions (semvar).
/// For example: 1.2.3, 6.5.4, 7.8.9-alpha, etc.
pub fn get_semver_regex() -> &'static Regex {
    SEMVER_REGEX.get_or_init(|| {
        Regex::new(r"^(?<major>[0-9]+).(?<minor>[0-9]+).(?<patch>[0-9]+)(?<pre>-[a-zA-Z]{1}[-0-9a-zA-Z.]+)?(?<build>\+[-0-9a-zA-Z.]+)?$",)
        .unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_alias() {
        assert!(is_alias_name("foo"));
        assert!(is_alias_name("foo.bar"));
        assert!(is_alias_name("foo/bar"));
        assert!(is_alias_name("foo-bar"));
        assert!(is_alias_name("foo_bar-baz"));
        assert!(is_alias_name("alpha.1"));
        assert!(is_alias_name("beta-0"));
        assert!(is_alias_name("rc-1.2.3"));
        assert!(is_alias_name("next-2023"));

        assert!(!is_alias_name("1.2.3"));
        assert!(!is_alias_name("1.2"));
        assert!(!is_alias_name("1"));
        assert!(!is_alias_name("1-3"));
    }

    #[test]
    fn cleans_string() {
        assert_eq!(clean_version_string("v1.2.3"), "1.2.3");
        assert_eq!(clean_version_string("V1.2.3"), "1.2.3");

        assert_eq!(clean_version_string("1.2.*"), "1.2");
        assert_eq!(clean_version_string("1.*.*"), "1");
        assert_eq!(clean_version_string("*"), "*");

        assert_eq!(clean_version_string(">= 1.2.3"), ">=1.2.3");
        assert_eq!(clean_version_string(">  1.2.3"), ">1.2.3");
        assert_eq!(clean_version_string("<1.2.3"), "<1.2.3");
        assert_eq!(clean_version_string("<=   1.2.3"), "<=1.2.3");

        assert_eq!(clean_version_string(">= v1.2.3"), ">=1.2.3");
        assert_eq!(clean_version_string(">  v1.2.3"), ">1.2.3");
        assert_eq!(clean_version_string("<v1.2.3"), "<1.2.3");
        assert_eq!(clean_version_string("<=   v1.2.3"), "<=1.2.3");

        assert_eq!(clean_version_string("1.2, 3"), "1.2,3");
        assert_eq!(clean_version_string("1,3, 4"), "1,3,4");
        assert_eq!(clean_version_string("1 2"), "1,2");
        assert_eq!(clean_version_string("1 && 2"), "1,2");
    }

    #[test]
    fn handles_commas() {
        assert_eq!(clean_version_string("1 2"), "1,2");
        assert_eq!(clean_version_string("1  2"), "1,2");
        assert_eq!(clean_version_string("1   2"), "1,2");
        assert_eq!(clean_version_string("1,2"), "1,2");
        assert_eq!(clean_version_string("1 ,2"), "1,2");
        assert_eq!(clean_version_string("1, 2"), "1,2");
        assert_eq!(clean_version_string("1 , 2"), "1,2");
        assert_eq!(clean_version_string("1  , 2"), "1,2");
        assert_eq!(clean_version_string("1,  2"), "1,2");
    }
}
