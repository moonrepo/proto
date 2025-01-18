use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::io;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoBuildError {
    #[diagnostic(code(proto::build::command_failed))]
    #[error("Failed to execute command {}.", .command.style(Style::Shell))]
    CommandFailed {
        command: String,
        error: Box<io::Error>,
    },

    #[diagnostic(code(proto::build::command_failed))]
    #[error("Command {} returned a {code} exit code.", .command.style(Style::Shell))]
    CommandNonZeroExit { command: String, code: i32 },

    #[diagnostic(code(proto::build::parse_version_failed))]
    #[error("Failed to parse version from {}.", .value.style(Style::Symbol))]
    VersionParseFailed {
        value: String,
        error: Box<semver::Error>,
    },

    #[diagnostic(code(proto::build::unmet_requirements))]
    #[error("Build requirements have not been met, unable to proceed.\nPlease satisfy the requirements before attempting the build again.")]
    RequirementsNotMet,
}
