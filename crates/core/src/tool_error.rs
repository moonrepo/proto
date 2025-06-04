use crate::config::PROTO_CONFIG_NAME;
use crate::config_error::ProtoConfigError;
use crate::layout::ProtoLayoutError;
use crate::tool_spec::Backend;
use crate::utils::archive::ProtoArchiveError;
use crate::utils::process::ProtoProcessError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::json::JsonError;
use std::path::PathBuf;
use thiserror::Error;
use warpgate::{Id, WarpgateClientError, WarpgatePluginError};

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoToolError {
    #[error(transparent)]
    Archive(#[from] Box<ProtoArchiveError>),

    #[error(transparent)]
    Client(#[from] Box<WarpgateClientError>),

    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[error(transparent)]
    Json(#[from] Box<JsonError>),

    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[error(transparent)]
    Process(#[from] Box<ProtoProcessError>),

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::tool::minimum_version_requirement))
    )]
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

    #[cfg_attr(feature = "miette", diagnostic(code(proto::tool::invalid_spec)))]
    #[error("Invalid version or requirement in tool specification {}.", .spec.style(Style::Hash))]
    InvalidVersionSpec {
        spec: String,
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::tool::invalid_inventory_dir))
    )]
    #[error("{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.", .dir.style(Style::Path))]
    RequiredAbsoluteInventoryDir { tool: String, dir: PathBuf },

    #[cfg_attr(feature = "miette", diagnostic(code(proto::tool::unknown_backend)))]
    #[error(
        "Unknown backend in tool specification {}. Only {} are supported.",
        .spec.style(Style::Hash),
        .backends.iter().map(|be| be.to_string().style(Style::Id)).collect::<Vec<_>>().join(", ")
    )]
    UnknownBackend {
        backends: Vec<Backend>,
        spec: String,
    },

    #[cfg_attr(feature = "miette", diagnostic(code(proto::tool::unknown_id)))]
    #[error(
        "Unable to proceed, {} is not a built-in plugin and has not been configured with {} in a {} file.\n\nLearn more about plugins: {}\nSearch community plugins: {}",
        .id.to_string().style(Style::Id),
        "[plugins]".style(Style::Property),
        PROTO_CONFIG_NAME.style(Style::File),
        "https://moonrepo.dev/docs/proto/plugins".style(Style::Url),
        format!("proto plugin search {}", .id).style(Style::Shell),
    )]
    UnknownTool { id: Id },
}

impl From<ProtoArchiveError> for ProtoToolError {
    fn from(e: ProtoArchiveError) -> ProtoToolError {
        ProtoToolError::Archive(Box::new(e))
    }
}

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

impl From<FsError> for ProtoToolError {
    fn from(e: FsError) -> ProtoToolError {
        ProtoToolError::Fs(Box::new(e))
    }
}

impl From<JsonError> for ProtoToolError {
    fn from(e: JsonError) -> ProtoToolError {
        ProtoToolError::Json(Box::new(e))
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
