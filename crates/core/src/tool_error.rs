use crate::config_error::ProtoConfigError;
use crate::layout::ProtoLayoutError;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;
use warpgate::{Id, WarpgateClientError, WarpgatePluginError};

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoToolError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Client(#[from] Box<WarpgateClientError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

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
    #[error("{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.", .dir.style(Style::Path))]
    RequiredAbsoluteInventoryDir { tool: String, dir: PathBuf },
}

unsafe impl Send for ProtoToolError {}
unsafe impl Sync for ProtoToolError {}

impl From<WarpgateClientError> for ProtoToolError {
    fn from(e: WarpgateClientError) -> ProtoToolError {
        ProtoToolError::Client(Box::new(e))
    }
}

impl From<ProtoConfigError> for ProtoToolError {
    fn from(e: ProtoConfigError) -> ProtoToolError {
        ProtoToolError::Config(Box::new(e))
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
