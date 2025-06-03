use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;
use warpgate::WarpgatePluginError;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoLocateError {
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::locate::missing_executable))
    )]
    #[error(
      "Unable to find an executable for {tool}, expected file {} does not exist.",
      .path.style(Style::Path),
    )]
    MissingToolExecutable { tool: String, path: PathBuf },
}

impl From<WarpgatePluginError> for ProtoLocateError {
    fn from(e: WarpgatePluginError) -> ProtoLocateError {
        ProtoLocateError::Plugin(Box::new(e))
    }
}
