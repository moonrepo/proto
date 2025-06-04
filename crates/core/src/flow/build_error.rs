use crate::utils::archive::ProtoArchiveError;
use crate::utils::process::ProtoProcessError;
use starbase_console::ConsoleError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::net::NetError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoBuildError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Archive(#[from] Box<ProtoArchiveError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Console(#[from] Box<ConsoleError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Net(#[from] Box<NetError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Process(#[from] Box<ProtoProcessError>),

    #[error(transparent)]
    System(#[from] Box<system_env::Error>),

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::install::build::parse_version_failed))
    )]
    #[error("Failed to parse version from {}.", .value.style(Style::Symbol))]
    FailedVersionParse {
        value: String,
        #[source]
        error: Box<semver::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::install::build::missing_builder))
    )]
    #[error("Builder {} has not been installed.",  .id.style(Style::Id))]
    MissingBuilder { id: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::install::build::missing_builder_exe))
    )]
    #[error("Executable {} from builder {} does not exist.", .exe.style(Style::Path), .id.style(Style::Id))]
    MissingBuilderExe { exe: PathBuf, id: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::install::build::unmet_requirements))
    )]
    #[error(
        "Build requirements have not been met, unable to proceed.\nPlease satisfy the requirements before attempting the build again."
    )]
    RequirementsNotMet,

    #[cfg_attr(feature = "miette", diagnostic(code(proto::install::build::cancelled)))]
    #[error("Build has been cancelled.")]
    Cancelled,
}

impl From<ProtoArchiveError> for ProtoBuildError {
    fn from(e: ProtoArchiveError) -> ProtoBuildError {
        ProtoBuildError::Archive(Box::new(e))
    }
}

impl From<ConsoleError> for ProtoBuildError {
    fn from(e: ConsoleError) -> ProtoBuildError {
        ProtoBuildError::Console(Box::new(e))
    }
}

impl From<FsError> for ProtoBuildError {
    fn from(e: FsError) -> ProtoBuildError {
        ProtoBuildError::Fs(Box::new(e))
    }
}

impl From<NetError> for ProtoBuildError {
    fn from(e: NetError) -> ProtoBuildError {
        ProtoBuildError::Net(Box::new(e))
    }
}

impl From<ProtoProcessError> for ProtoBuildError {
    fn from(e: ProtoProcessError) -> ProtoBuildError {
        ProtoBuildError::Process(Box::new(e))
    }
}

impl From<system_env::Error> for ProtoBuildError {
    fn from(e: system_env::Error) -> ProtoBuildError {
        ProtoBuildError::System(Box::new(e))
    }
}
