mod resolved_spec;
mod spec_error;
mod syntax;
mod syntax_parser;
mod syntax_traits;
mod unresolved_spec;

pub use resolved_spec::*;
pub use spec_error::*;
pub use syntax::*;
#[doc(hidden)]
pub use syntax_parser::*;
pub use syntax_traits::*;
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

/// Returns true if the provided value looks like a semver version string,
/// instead of a calendar version string. This is used to determine which parser to use.
pub fn is_semver_like(value: &str) -> bool {
    static SEMVER_REGEX: OnceLock<Regex> = OnceLock::new();

    SEMVER_REGEX
        .get_or_init(|| Regex::new(r"-?(v|V)?[0-9]+\.[0-9]+(\.[0-9]+)?-?").unwrap())
        .is_match(value)
}
