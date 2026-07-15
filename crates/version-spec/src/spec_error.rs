pub type ParseError = pest::error::Error<crate::syntax_parser::Rule>;

#[derive(thiserror::Error, Debug)]
pub enum SpecError {
    #[error("Failed to parse a version.")]
    FailedVersionParse {
        #[source]
        error: ParseError,
    },

    #[error("Failed to parse a version requirement.")]
    FailedVersionRequirementParse {
        #[source]
        error: ParseError,
    },

    #[error("Failed to parse a version range.")]
    FailedVersionRangeParse {
        #[source]
        error: ParseError,
    },

    #[error("Invalid calver (calendar version) format.")]
    InvalidCalverFormat,

    #[error("Invalid semver (semantic version) format.")]
    InvalidSemverFormat,

    #[error("Requirement operator found in an invalid position.")]
    InvalidParseRequirement,

    #[error("Invalid calver year, must be a number.")]
    InvalidYear,

    #[error("Missing major number for semantic versions, or year for calendar versions.")]
    MissingParseMajorPart,

    #[error("Unknown version format `{0}`. Must be a semantic or calendar based format.")]
    UnknownResolvedFormat(String),

    #[error("Unknown character `{0}` in version string!")]
    UnknownParseChar(char),

    #[error(transparent)]
    Semver(#[from] semver::Error),
}
