use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum WarpgateClientError {
    #[diagnostic(code(plugin::http_client::request_failed))]
    #[error("Failed to make HTTP request for {}.", .url.style(Style::Url))]
    Http {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[diagnostic(code(plugin::http_client::request_failed))]
    #[error(
      "Failed to make HTTP request for {}: {}",
      .url.style(Style::Url),
      .error.style(Style::MutedLight),
    )]
    HttpMiddleware { url: String, error: String },

    #[diagnostic(code(plugin::http_client::invalid_cert))]
    #[error("Invalid certificate {}.", .path.style(Style::Path))]
    InvalidCert {
        path: PathBuf,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[diagnostic(code(plugin::http_client::invalid_proxy))]
    #[error("Invalid proxy {}.", .url.style(Style::Url))]
    InvalidProxy {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },
}
