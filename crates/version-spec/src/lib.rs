mod resolved_spec;
mod unresolved_spec;
mod version_types;

pub use resolved_spec::*;
pub use unresolved_spec::*;
pub use version_types::*;

use regex::Regex;

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

pub fn is_calver_like<T: AsRef<str>>(value: T) -> bool {
    let value = value.as_ref();

    !value.is_empty() && get_calver_regex().is_match(value)
}

/// Cleans a potential version string by removing a leading `v` or `V`,
/// removing each occurence of `.*`, and removing invalid spaces.
pub fn clean_version_string<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref().trim();

    if value.contains("||") {
        return value
            .split("||")
            .map(clean_version_string)
            .collect::<Vec<_>>()
            .join(" || ");
    }

    let mut version = value.replace(".*", "").replace("&&", ",");

    // Remove a leading "v" or "V" from a version string.
    #[allow(clippy::assigning_clones)]
    if version.starts_with('v') || version.starts_with('V') {
        version = version[1..].to_owned();
    }

    // Remove invalid space after <, <=, >, >=.
    let version = regex::Regex::new(r"([><]=?)[ ]*v?([0-9])")
        .unwrap()
        .replace_all(&version, "$1$2");

    // Replace spaces with commas
    regex::Regex::new("[, ]+")
        .unwrap()
        .replace_all(&version, ",")
        .to_string()
}

/// Get a regex pattern that matches calendar versions (calver).
/// For example: 2024-02-26, 2024-12, 2024-01-alpha, etc.
pub fn get_calver_regex() -> Regex {
    Regex::new(
        r"^
        (?<year>[0-9]{1,4})(?
            -(?<month>[0-9]{1,2})(?
                -(?<day>[0-9]{1,2})(?
                    (_|.)(?<micro>[0-9]+)
                )?
            )?
        )?
        (?<pre>-[-0-9a-zA-Z.]+)?
        $",
    )
    .unwrap()
}

/// Get a regex pattern that matches semantic versions (semvar).
/// For example: 1.2.3, 6.5.4, 7.8.9-alpha, etc.
pub fn get_semver_regex() -> Regex {
    Regex::new(
        r"^
        (?<major>[0-9]+).(?<minor>[0-9]+).(?<patch>[0-9]+)
        (?<pre>-[-0-9a-zA-Z.]+)?
        (?<build>+[-0-9a-zA-Z.]+)?
        $",
    )
    .unwrap()
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
