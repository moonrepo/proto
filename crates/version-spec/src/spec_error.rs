#[derive(thiserror::Error, Debug)]
pub enum SpecError {
    #[error("Invalid calver (calendar version) format.")]
    CalverInvalidFormat,

    #[error("Unknown version format `{0}`. Must be a semantic or calendar based format.")]
    ResolvedUnknownFormat(String),

    #[error("Requirement operator found in an invalid position")]
    ParseInvalidReq,

    #[error("Unknown character `{0}` in version string!")]
    ParseUnknownChar(char),

    #[error("Missing major number for semantic versions, or year for calendar versions.")]
    ParseMissingMajorPart,

    #[error(transparent)]
    Semver(#[from] semver::Error),
}
