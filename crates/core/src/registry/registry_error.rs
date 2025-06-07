use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::json::JsonError;
use thiserror::Error;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoRegistryError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Json(#[from] Box<JsonError>),

    #[diagnostic(code(proto::registry::parse_failed))]
    #[error("Failed to parse registry plugin data.")]
    FailedParse {
        #[source]
        error: Box<reqwest::Error>,
    },

    #[diagnostic(code(proto::registry::request_failed))]
    #[error("Failed to request plugins from registry {}.", .url.style(Style::Url))]
    FailedRequest {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },
}

impl From<FsError> for ProtoRegistryError {
    fn from(e: FsError) -> ProtoRegistryError {
        ProtoRegistryError::Fs(Box::new(e))
    }
}

impl From<JsonError> for ProtoRegistryError {
    fn from(e: JsonError) -> ProtoRegistryError {
        ProtoRegistryError::Json(Box::new(e))
    }
}
