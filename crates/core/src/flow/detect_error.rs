use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoDetectError {
    #[diagnostic(code(proto::detect::invalid_version))]
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

    #[diagnostic(code(proto::detect::failed), help = "Has the tool been installed?")]
    #[error(
        "Failed to detect an applicable version to run {tool} with. Try pinning a version with {} or explicitly passing the version as an argument or environment variable.",
        "proto pin".style(Style::Shell),
    )]
    FailedVersionDetect { tool: String },
}
