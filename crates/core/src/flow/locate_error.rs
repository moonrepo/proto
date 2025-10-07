use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::path::PathBuf;
use thiserror::Error;
use warpgate::WarpgatePluginError;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoLocateError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(code(proto::locate::missing_executable))]
    #[error(
      "Unable to find an executable for {tool}, expected file {} does not exist.",
      .path.style(Style::Path),
    )]
    MissingToolExecutable { tool: String, path: PathBuf },

    #[diagnostic(code(proto::locate::no_primary_executable))]
    #[error(
      "{tool} does not support a primary (default) executable. You can run a secondary executable by passing {} with the executable name.",
      "--exe".style(Style::Shell),
    )]
    NoPrimaryExecutable { tool: String },
}

impl From<FsError> for ProtoLocateError {
    fn from(e: FsError) -> ProtoLocateError {
        ProtoLocateError::Fs(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoLocateError {
    fn from(e: WarpgatePluginError) -> ProtoLocateError {
        ProtoLocateError::Plugin(Box::new(e))
    }
}
