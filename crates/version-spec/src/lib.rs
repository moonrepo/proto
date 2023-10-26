mod resolved_spec;
mod unresolved_spec;

use once_cell::sync::Lazy;
use regex::Regex;

pub use resolved_spec::*;
pub use unresolved_spec::*;

/// Aliases are words that map to version. For example, "latest" -> "1.2.3".
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

static CLEAN_VERSION: Lazy<Regex> = Lazy::new(|| Regex::new(r"([><]=?)\s+(\d)").unwrap());

pub fn clean_version_string<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref().trim().replace(".*", "");
    let mut version = value.as_str();

    // Remove a leading "v" or "V" from a version string.
    if version.starts_with('v') || version.starts_with('V') {
        version = &version[1..];
    }

    // Remove invalid space after <, <=, >, >=.
    CLEAN_VERSION.replace_all(version, "$1$2").to_string()
}
