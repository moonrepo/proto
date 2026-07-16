pub type ParseError = pest::error::Error<crate::syntax_parser::Rule>;

#[derive(thiserror::Error, Debug)]
pub enum SpecError {
    #[error("Failed to parse a version.")]
    FailedVersionParse {
        #[source]
        error: Box<ParseError>,
    },

    #[error("Failed to parse a version requirement.")]
    FailedVersionRequirementParse {
        #[source]
        error: Box<ParseError>,
    },

    #[error("Failed to parse a version range.")]
    FailedVersionRangeParse {
        #[source]
        error: Box<ParseError>,
    },
}
