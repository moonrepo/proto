use crate::config_error::ProtoConfigError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::path::PathBuf;
use thiserror::Error;
use warpgate::WarpgatePluginError;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoDetectError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[cfg_attr(feature = "miette", diagnostic(code(proto::detect::invalid_version)))]
    #[error(
      "Invalid version or requirement {} detected from {}.",
      .version.style(Style::Hash),
      .path.style(Style::Path),
    )]
    InvalidDetectedVersionSpec {
        #[source]
        error: Box<version_spec::SpecError>,
        path: PathBuf,
        version: String,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::detect::failed), help = "Has the tool been installed?")
    )]
    #[error(
        "Failed to detect an applicable version to run {tool} with. Try pinning a version with {} or explicitly passing the version as an argument or environment variable.",
        "proto pin".style(Style::Shell),
    )]
    FailedVersionDetect { tool: String },
}

impl From<ProtoConfigError> for ProtoDetectError {
    fn from(e: ProtoConfigError) -> ProtoDetectError {
        ProtoDetectError::Config(Box::new(e))
    }
}

impl From<FsError> for ProtoDetectError {
    fn from(e: FsError) -> ProtoDetectError {
        ProtoDetectError::Fs(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoDetectError {
    fn from(e: WarpgatePluginError) -> ProtoDetectError {
        ProtoDetectError::Plugin(Box::new(e))
    }
}
