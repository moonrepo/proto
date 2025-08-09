use crate::config_error::ProtoConfigError;
use crate::layout::ProtoLayoutError;
use crate::utils::archive::ProtoArchiveError;
use crate::utils::process::ProtoProcessError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::path::PathBuf;
use thiserror::Error;
use warpgate::{Id, WarpgateHttpClientError, WarpgatePluginError};

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoToolError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Archive(#[from] Box<ProtoArchiveError>),

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
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] Box<ProtoProcessError>),

    #[diagnostic(code(proto::tool::minimum_version_requirement))]
    #[error(
        "Unable to use the {tool} plugin with identifier {}, as it requires a minimum proto version of {}, but found {} instead.",
        .id.to_string().style(Style::Id),
        .expected.style(Style::Hash),
        .actual.style(Style::Hash)
    )]
    InvalidMinimumVersion {
        tool: String,
        id: Id,
        expected: String,
        actual: String,
    },

    #[diagnostic(code(proto::tool::invalid_inventory_dir))]
    #[error(
        "{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.",
        .dir.style(Style::Path),
    )]
    RequiredAbsoluteInventoryDir { tool: String, dir: PathBuf },
}

impl From<ProtoArchiveError> for ProtoToolError {
    fn from(e: ProtoArchiveError) -> ProtoToolError {
        ProtoToolError::Archive(Box::new(e))
    }
}

impl From<WarpgateHttpClientError> for ProtoToolError {
    fn from(e: WarpgateHttpClientError) -> ProtoToolError {
        ProtoToolError::HttpClient(Box::new(e))
    }
}

impl From<ProtoConfigError> for ProtoToolError {
    fn from(e: ProtoConfigError) -> ProtoToolError {
        ProtoToolError::Config(Box::new(e))
    }
}

impl From<FsError> for ProtoToolError {
    fn from(e: FsError) -> ProtoToolError {
        ProtoToolError::Fs(Box::new(e))
    }
}

impl From<ProtoLayoutError> for ProtoToolError {
    fn from(e: ProtoLayoutError) -> ProtoToolError {
        ProtoToolError::Layout(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoToolError {
    fn from(e: WarpgatePluginError) -> ProtoToolError {
        ProtoToolError::Plugin(Box::new(e))
    }
}

impl From<ProtoProcessError> for ProtoToolError {
    fn from(e: ProtoProcessError) -> ProtoToolError {
        ProtoToolError::Process(Box::new(e))
    }
}
