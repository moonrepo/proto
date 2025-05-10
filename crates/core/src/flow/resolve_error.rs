use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoResolveError {
    #[diagnostic(code(proto::resolve::offline::version_required))]
    #[error(
        "Internet connection required to load and resolve a valid version. To work around this:\n - Pass a fully-qualified version explicitly: {}\n - Execute the non-shim binaries instead: {}",
        .command.style(Style::Shell),
        .bin_dir.style(Style::Path)
    )]
    RequiredInternetConnectionForVersion { command: String, bin_dir: PathBuf },

    #[diagnostic(code(proto::resolve::invalid_version))]
    #[error("Invalid version or requirement {}.", .version.style(Style::Hash))]
    InvalidVersionSpec {
        version: String,
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[diagnostic(
        code(proto::resolve::undetected_version),
        help = "Has the tool been installed?"
    )]
    #[error(
        "Failed to detect an applicable version to run {tool} with. Try pinning a version with {} or explicitly passing the version as an argument or environment variable.",
        "proto pin".style(Style::Shell),
    )]
    FailedVersionDetect { tool: String },

    #[diagnostic(
        code(proto::resolve::unresolved_version),
        help = "Does this version exist and has it been released?"
    )]
    #[error(
        "Failed to resolve {} to a valid supported version for {tool}.",
        .version.style(Style::Hash),
    )]
    FailedVersionResolve { tool: String, version: String },
}
