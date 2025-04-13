use miette::Diagnostic;
use starbase_styles::{Style, Stylize, apply_style_tags};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoInstallError {
    #[diagnostic(code(proto::install::failed))]
    #[error("Failed to install {tool}. {}", apply_style_tags(.error))]
    FailedInstall { tool: String, error: String },

    #[diagnostic(code(proto::uninstall::failed))]
    #[error("Failed to uninstall {tool}. {}", apply_style_tags(.error))]
    FailedUninstall { tool: String, error: String },

    #[diagnostic(code(proto::install::invalid_checksum))]
    #[error(
        "Checksum has failed for {}, which was verified using {}.",
        .download.style(Style::Path),
        .checksum.style(Style::Path),
    )]
    InvalidChecksum {
        checksum: PathBuf,
        download: PathBuf,
    },

    #[diagnostic(code(proto::install::prebuilt_unsupported))]
    #[error("Downloading a pre-built is not supported for {tool}. Try building from source by passing {}.", "--build".style(Style::Shell))]
    UnsupportedDownloadPrebuilt { tool: String },

    #[diagnostic(code(proto::install::build_unsupported))]
    #[error("Building from source is not supported for {tool}. Try downloading a pre-built by passing {}.", "--no-build".style(Style::Shell))]
    UnsupportedBuildFromSource { tool: String },
}
