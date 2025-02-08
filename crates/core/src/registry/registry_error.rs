use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoRegistryError {
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
