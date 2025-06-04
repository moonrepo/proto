use crate::config_error::ProtoConfigError;
use crate::layout::ProtoLayoutError;
use crate::tool_error::ProtoToolError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;
use warpgate::WarpgatePluginError;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoResolveError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Tool(#[from] Box<ProtoToolError>),

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::resolve::offline::version_required))
    )]
    #[error(
        "Internet connection required to load and resolve a valid version. To work around this:\n - Pass a fully-qualified version explicitly: {}\n - Execute the non-shim binaries instead: {}",
        .command.style(Style::Shell),
        .bin_dir.style(Style::Path)
    )]
    RequiredInternetConnectionForVersion { command: String, bin_dir: PathBuf },

    #[cfg_attr(feature = "miette", diagnostic(code(proto::resolve::invalid_version)))]
    #[error("Invalid version or requirement {}.", .version.style(Style::Hash))]
    InvalidVersionSpec {
        version: String,
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(proto::resolve::undetected_version),
            help = "Has the tool been installed?"
        )
    )]
    #[error(
        "Failed to detect an applicable version to run {tool} with. Try pinning a version with {} or explicitly passing the version as an argument or environment variable.",
        "proto pin".style(Style::Shell),
    )]
    FailedVersionDetect { tool: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(proto::resolve::unresolved_version),
            help = "Does this version exist and has it been released?"
        )
    )]
    #[error(
        "Failed to resolve {} to a valid supported version for {tool}.",
        .version.style(Style::Hash),
    )]
    FailedVersionResolve { tool: String, version: String },
}

impl From<ProtoConfigError> for ProtoResolveError {
    fn from(e: ProtoConfigError) -> ProtoResolveError {
        ProtoResolveError::Config(Box::new(e))
    }
}

impl From<ProtoLayoutError> for ProtoResolveError {
    fn from(e: ProtoLayoutError) -> ProtoResolveError {
        ProtoResolveError::Layout(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoResolveError {
    fn from(e: WarpgatePluginError) -> ProtoResolveError {
        ProtoResolveError::Plugin(Box::new(e))
    }
}

impl From<ProtoToolError> for ProtoResolveError {
    fn from(e: ProtoToolError) -> ProtoResolveError {
        ProtoResolveError::Tool(Box::new(e))
    }
}
