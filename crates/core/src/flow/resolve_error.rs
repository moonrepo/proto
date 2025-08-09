use crate::config_error::ProtoConfigError;
use crate::flow::lock::ProtoLockError;
use crate::layout::ProtoLayoutError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::path::PathBuf;
use thiserror::Error;
use warpgate::{WarpgateHttpClientError, WarpgatePluginError};

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoResolveError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    HttpClient(#[from] Box<WarpgateHttpClientError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Lock(#[from] Box<ProtoLockError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(code(proto::resolve::offline::version_required))]
    #[error(
        "Internet connection required to load and resolve a valid version. To work around this:\n - Pass a fully-qualified version explicitly: {}\n - Execute the non-shim binaries instead: {}",
        .command.style(Style::Shell),
        .bin_dir.style(Style::Path)
    )]
    RequiredInternetConnectionForVersion { command: String, bin_dir: PathBuf },

    #[diagnostic(code(proto::resolve::invalid_version))]
    #[error("Invalid version or requirement in tool specification {}.", .version.style(Style::Hash))]
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

impl From<WarpgateHttpClientError> for ProtoResolveError {
    fn from(e: WarpgateHttpClientError) -> ProtoResolveError {
        ProtoResolveError::HttpClient(Box::new(e))
    }
}

impl From<ProtoConfigError> for ProtoResolveError {
    fn from(e: ProtoConfigError) -> ProtoResolveError {
        ProtoResolveError::Config(Box::new(e))
    }
}

impl From<FsError> for ProtoResolveError {
    fn from(e: FsError) -> ProtoResolveError {
        ProtoResolveError::Fs(Box::new(e))
    }
}

impl From<ProtoLayoutError> for ProtoResolveError {
    fn from(e: ProtoLayoutError) -> ProtoResolveError {
        ProtoResolveError::Layout(Box::new(e))
    }
}

impl From<ProtoLockError> for ProtoResolveError {
    fn from(e: ProtoLockError) -> ProtoResolveError {
        ProtoResolveError::Lock(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoResolveError {
    fn from(e: WarpgatePluginError) -> ProtoResolveError {
        ProtoResolveError::Plugin(Box::new(e))
    }
}
