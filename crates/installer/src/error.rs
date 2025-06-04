use starbase_archive::ArchiveError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::net::NetError;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoInstallerError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Archive(#[from] Box<ArchiveError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Net(#[from] Box<NetError>),

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::installer::invalid_platform))
    )]
    #[error("Unable to download and install proto, unsupported platform {} + {}.", .os, .arch)]
    InvalidPlatform { arch: String, os: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::installer::download_failed))
    )]
    #[error("Failed to download archive {}.", .url.style(Style::Url))]
    FailedDownload {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(proto::installer::not_available),
            help("A release may be in progress, please try again later!"),
            url("https://github.com/moonrepo/proto/releases")
        )
    )]
    #[error(
        "Download for proto v{} is not available.\n{}",
        .version,
        format!("Status: {}", .status).style(Style::MutedLight),
    )]
    DownloadNotAvailable { version: String, status: String },
}

impl From<ArchiveError> for ProtoInstallerError {
    fn from(e: ArchiveError) -> ProtoInstallerError {
        ProtoInstallerError::Archive(Box::new(e))
    }
}

impl From<FsError> for ProtoInstallerError {
    fn from(e: FsError) -> ProtoInstallerError {
        ProtoInstallerError::Fs(Box::new(e))
    }
}

impl From<NetError> for ProtoInstallerError {
    fn from(e: NetError) -> ProtoInstallerError {
        ProtoInstallerError::Net(Box::new(e))
    }
}
