#[derive(thiserror::Error, Debug)]
pub enum SpecError {
    #[error("Invalid calver (calendar version) format.")]
    InvalidCalverFormat,

    #[error("Requirement operator found in an invalid position.")]
    InvalidParseRequirement,

    #[error("Missing major number for semantic versions, or year for calendar versions.")]
    MissingParseMajorPart,

    #[error("Unknown version format `{0}`. Must be a semantic or calendar based format.")]
    UnknownResolvedFormat(String),

    #[error("Unknown character `{0}` in version string!")]
    UnknownParseChar(char),

    #[error(transparent)]
    Semver(#[from] semver::Error),
}
