use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;
use warpgate::WarpgatePluginError;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoLocateError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(code(proto::locate::missing_executable))]
    #[error(
      "Unable to find an executable for {tool}, expected file {} does not exist.",
      .path.style(Style::Path),
    )]
    MissingToolExecutable { tool: String, path: PathBuf },
}

unsafe impl Send for ProtoLocateError {}
unsafe impl Sync for ProtoLocateError {}

impl From<WarpgatePluginError> for ProtoLocateError {
    fn from(e: WarpgatePluginError) -> ProtoLocateError {
        ProtoLocateError::Plugin(Box::new(e))
    }
}
