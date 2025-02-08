use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use starbase_utils::net::NetError;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoInstallerError {
    #[error(transparent)]
    Net(#[from] Box<NetError>),

    #[diagnostic(code(proto::installer::invalid_platform))]
    #[error("Unable to download and install proto, unsupported platform {} + {}.", .os, .arch)]
    InvalidPlatform { arch: String, os: String },

    #[diagnostic(code(proto::installer::download_failed))]
    #[error("Failed to download archive {}.", .url.style(Style::Url))]
    FailedDownload {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[diagnostic(
        code(proto::installer::not_available),
        help("A release may be in progress, please try again later!"),
        url("https://github.com/moonrepo/proto/releases")
    )]
    #[error(
        "Download for proto v{} is not available.\n{}",
        .version,
        format!("Status: {}", .status).style(Style::MutedLight),
    )]
    DownloadNotAvailable { version: String, status: String },
}
