use crate::config::PROTO_CONFIG_NAME;
use crate::config_error::ProtoConfigError;
use crate::flow::resolve::ProtoResolveError;
use crate::tool_error::ProtoToolError;
use starbase_styles::{Style, Stylize};
use starbase_utils::json::JsonError;
use starbase_utils::toml::TomlError;
use starbase_utils::yaml::YamlError;
use thiserror::Error;
use warpgate::{Id, WarpgateLoaderError, WarpgatePluginError};

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoLoaderError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Json(#[from] Box<JsonError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Loader(#[from] Box<WarpgateLoaderError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Resolve(#[from] Box<ProtoResolveError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Toml(#[from] Box<TomlError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Tool(#[from] Box<ProtoToolError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Yaml(#[from] Box<YamlError>),

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

unsafe impl Send for ProtoLoaderError {}
unsafe impl Sync for ProtoLoaderError {}

impl From<ProtoConfigError> for ProtoLoaderError {
    fn from(e: ProtoConfigError) -> ProtoLoaderError {
        ProtoLoaderError::Config(Box::new(e))
    }
}

impl From<JsonError> for ProtoLoaderError {
    fn from(e: JsonError) -> ProtoLoaderError {
        ProtoLoaderError::Json(Box::new(e))
    }
}

impl From<WarpgateLoaderError> for ProtoLoaderError {
    fn from(e: WarpgateLoaderError) -> ProtoLoaderError {
        ProtoLoaderError::Loader(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoLoaderError {
    fn from(e: WarpgatePluginError) -> ProtoLoaderError {
        ProtoLoaderError::Plugin(Box::new(e))
    }
}

impl From<ProtoResolveError> for ProtoLoaderError {
    fn from(e: ProtoResolveError) -> ProtoLoaderError {
        ProtoLoaderError::Resolve(Box::new(e))
    }
}

impl From<TomlError> for ProtoLoaderError {
    fn from(e: TomlError) -> ProtoLoaderError {
        ProtoLoaderError::Toml(Box::new(e))
    }
}

impl From<ProtoToolError> for ProtoLoaderError {
    fn from(e: ProtoToolError) -> ProtoLoaderError {
        ProtoLoaderError::Tool(Box::new(e))
    }
}

impl From<YamlError> for ProtoLoaderError {
    fn from(e: YamlError) -> ProtoLoaderError {
        ProtoLoaderError::Yaml(Box::new(e))
    }
}
