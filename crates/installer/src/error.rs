use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoInstallerError {
    #[diagnostic(code(proto::installer::invalid_platform))]
    #[error("Unable to download and install proto, unsupported platform {} + {}.", .os, .arch)]
    InvalidPlatform { arch: String, os: String },

    #[diagnostic(code(proto::installer::download_failed))]
    #[error("Failed to download archive {}.", .url.style(Style::Url))]
    DownloadFailed {
        url: String,
        #[source]
        error: reqwest::Error,
    },
}
