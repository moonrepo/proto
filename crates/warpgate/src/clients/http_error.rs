use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::path::PathBuf;
use thiserror::Error;

/// HTTP(S) client errors.
#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum WarpgateHttpClientError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::http_client::create_failed))
    )]
    #[error("Failed to create HTTP client.")]
    Client {
        #[source]
        error: Box<reqwest::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::http_client::request_failed))
    )]
    #[error("Failed to make HTTP request for {}.", .url.style(Style::Url))]
    Http {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::http_client::request_failed))
    )]
    #[error(
      "Failed to make HTTP request for {}: {}",
      .url.style(Style::Url),
      .error.style(Style::MutedLight),
    )]
    HttpMiddleware { url: String, error: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::http_client::invalid_cert))
    )]
    #[error("Invalid certificate {}.", .path.style(Style::Path))]
    InvalidCert {
        path: PathBuf,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::http_client::invalid_proxy))
    )]
    #[error("Invalid proxy {}.", .url.style(Style::Url))]
    InvalidProxy {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::http_client::invalid_user_agent))
    )]
    #[error(
        "Invalid user agent {}. Must not be empty, and must not contain control characters.",
        .value.style(Style::Symbol)
    )]
    InvalidUserAgent { value: String },
}

impl From<FsError> for WarpgateHttpClientError {
    fn from(e: FsError) -> WarpgateHttpClientError {
        WarpgateHttpClientError::Fs(Box::new(e))
    }
}
