use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoBuildError {
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
