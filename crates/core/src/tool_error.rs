use crate::config::PROTO_CONFIG_NAME;
use crate::config_error::ProtoConfigError;
use crate::flow::resolve::ProtoResolveError;
use crate::layout::ProtoLayoutError;
use crate::tool_spec::Backend;
use crate::utils::archive::ProtoArchiveError;
use crate::utils::process::ProtoProcessError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::json::JsonError;
use starbase_utils::toml::TomlError;
use starbase_utils::yaml::YamlError;
use std::path::PathBuf;
use thiserror::Error;
use warpgate::{Id, WarpgateClientError, WarpgateLoaderError, WarpgatePluginError};

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoToolError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Archive(#[from] Box<ProtoArchiveError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Client(#[from] Box<WarpgateClientError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Json(#[from] Box<JsonError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Loader(#[from] Box<WarpgateLoaderError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Process(#[from] Box<ProtoProcessError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Resolve(#[from] Box<ProtoResolveError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Toml(#[from] Box<TomlError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Yaml(#[from] Box<YamlError>),

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

    #[diagnostic(code(proto::tool::invalid_spec))]
    #[error("Invalid version or requirement in tool specification {}.", .spec.style(Style::Hash))]
    InvalidVersionSpec {
        spec: String,
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[diagnostic(code(proto::tool::invalid_inventory_dir))]
    #[error("{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.", .dir.style(Style::Path))]
    RequiredAbsoluteInventoryDir { tool: String, dir: PathBuf },

    #[diagnostic(code(proto::tool::unknown_backend))]
    #[error(
        "Unknown backend in tool specification {}. Only {} are supported.",
        .spec.style(Style::Hash),
        .backends.iter().map(|be| be.to_string().style(Style::Id)).collect::<Vec<_>>().join(", ")
    )]
    UnknownBackend {
        backends: Vec<Backend>,
        spec: String,
    },

    #[diagnostic(code(proto::tool::unknown_id))]
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

impl From<WarpgateLoaderError> for ProtoToolError {
    fn from(e: WarpgateLoaderError) -> ProtoToolError {
        ProtoToolError::Loader(Box::new(e))
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

impl From<ProtoResolveError> for ProtoToolError {
    fn from(e: ProtoResolveError) -> ProtoToolError {
        ProtoToolError::Resolve(Box::new(e))
    }
}

impl From<TomlError> for ProtoToolError {
    fn from(e: TomlError) -> ProtoToolError {
        ProtoToolError::Toml(Box::new(e))
    }
}

impl From<YamlError> for ProtoToolError {
    fn from(e: YamlError) -> ProtoToolError {
        ProtoToolError::Yaml(Box::new(e))
    }
}
