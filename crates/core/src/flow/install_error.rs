use super::build_error::ProtoBuildError;
use crate::checksum::ProtoChecksumError;
use crate::config_error::ProtoConfigError;
use crate::utils::archive::ProtoArchiveError;
use starbase_styles::{Style, Stylize, apply_style_tags};
use starbase_utils::fs::FsError;
use starbase_utils::net::NetError;
use std::path::PathBuf;
use thiserror::Error;
use warpgate::{WarpgateClientError, WarpgatePluginError};

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoInstallError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Archive(#[from] Box<ProtoArchiveError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Build(#[from] Box<ProtoBuildError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Checksum(#[from] Box<ProtoChecksumError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Client(#[from] Box<WarpgateClientError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Net(#[from] Box<NetError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(code(proto::install::failed))]
    #[error("Failed to install {tool}. {}", apply_style_tags(.error))]
    FailedInstall { tool: String, error: String },

    #[diagnostic(code(proto::uninstall::failed))]
    #[error("Failed to uninstall {tool}. {}", apply_style_tags(.error))]
    FailedUninstall { tool: String, error: String },

    #[diagnostic(code(proto::install::invalid_checksum))]
    #[error(
        "Checksum has failed for {}, which was verified using {}.",
        .download.style(Style::Path),
        .checksum.style(Style::Path),
    )]
    InvalidChecksum {
        checksum: PathBuf,
        download: PathBuf,
    },

    #[diagnostic(
        code(proto::install::mismatched_checksum),
        help = "Is this install legitimate?"
    )]
    #[error(
        "Checksum mismatch! Received {} but expected {}.",
        .checksum.style(Style::Hash),
        .lockfile_checksum.style(Style::Hash),
    )]
    MismatchedChecksum {
        checksum: String,
        lockfile_checksum: String,
    },

    #[diagnostic(
        code(proto::install::mismatched_checksum),
        help = "Is this install legitimate?"
    )]
    #[error(
        "Checksum mismatch for {}! Received {} but expected {}.",
        .source_url.style(Style::Url),
        .checksum.style(Style::Hash),
        .lockfile_checksum.style(Style::Hash),
    )]
    MismatchedChecksumWithSource {
        checksum: String,
        lockfile_checksum: String,
        source_url: String,
    },

    #[diagnostic(code(proto::install::prebuilt_unsupported))]
    #[error("Downloading a pre-built is not supported for {tool}. Try building from source by passing {}.", "--build".style(Style::Shell))]
    UnsupportedDownloadPrebuilt { tool: String },

    #[diagnostic(code(proto::install::build_unsupported))]
    #[error("Building from source is not supported for {tool}. Try downloading a pre-built by passing {}.", "--no-build".style(Style::Shell))]
    UnsupportedBuildFromSource { tool: String },

    #[diagnostic(code(proto::offline))]
    #[error("Internet connection required, unable to download, install, or run tools.")]
    RequiredInternetConnection,
}

impl From<ProtoArchiveError> for ProtoInstallError {
    fn from(e: ProtoArchiveError) -> ProtoInstallError {
        ProtoInstallError::Archive(Box::new(e))
    }
}

impl From<ProtoBuildError> for ProtoInstallError {
    fn from(e: ProtoBuildError) -> ProtoInstallError {
        ProtoInstallError::Build(Box::new(e))
    }
}

impl From<ProtoChecksumError> for ProtoInstallError {
    fn from(e: ProtoChecksumError) -> ProtoInstallError {
        ProtoInstallError::Checksum(Box::new(e))
    }
}

impl From<WarpgateClientError> for ProtoInstallError {
    fn from(e: WarpgateClientError) -> ProtoInstallError {
        ProtoInstallError::Client(Box::new(e))
    }
}

impl From<ProtoConfigError> for ProtoInstallError {
    fn from(e: ProtoConfigError) -> ProtoInstallError {
        ProtoInstallError::Config(Box::new(e))
    }
}

impl From<FsError> for ProtoInstallError {
    fn from(e: FsError) -> ProtoInstallError {
        ProtoInstallError::Fs(Box::new(e))
    }
}

impl From<NetError> for ProtoInstallError {
    fn from(e: NetError) -> ProtoInstallError {
        ProtoInstallError::Net(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoInstallError {
    fn from(e: WarpgatePluginError) -> ProtoInstallError {
        ProtoInstallError::Plugin(Box::new(e))
    }
}
