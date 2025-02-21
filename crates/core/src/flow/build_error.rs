use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoBuildError {
    #[diagnostic(code(proto::install::build::parse_version_failed))]
    #[error("Failed to parse version from {}.", .value.style(Style::Symbol))]
    FailedVersionParse {
        value: String,
        #[source]
        error: Box<semver::Error>,
    },

    #[diagnostic(code(proto::install::build::missing_builder))]
    #[error("Builder {} has not been installed.",  .id.style(Style::Id))]
    MissingBuilder { id: String },

    #[diagnostic(code(proto::install::build::missing_builder_exe))]
    #[error("Executable {} from builder {} does not exist.", .exe.style(Style::Path), .id.style(Style::Id))]
    MissingBuilderExe { exe: PathBuf, id: String },

    #[diagnostic(code(proto::install::build::unmet_requirements))]
    #[error(
        "Build requirements have not been met, unable to proceed.\nPlease satisfy the requirements before attempting the build again."
    )]
    RequirementsNotMet,

    #[diagnostic(code(proto::install::build::cancelled))]
    #[error("Build has been cancelled.")]
    Cancelled,
}
