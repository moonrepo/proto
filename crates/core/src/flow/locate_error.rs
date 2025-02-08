use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoLocateError {
    #[diagnostic(code(proto::locate::missing_executable))]
    #[error(
      "Unable to find an executable for {tool}, expected file {} does not exist.",
      .path.style(Style::Path),
    )]
    MissingToolExecutable { tool: String, path: PathBuf },
}
