use crate::config_error::ProtoConfigError;
use crate::flow::install::ProtoInstallError;
use crate::flow::link::ProtoLinkError;
use crate::flow::locate::ProtoLocateError;
use crate::flow::resolve::ProtoResolveError;
use crate::layout::ProtoLayoutError;
use starbase_utils::json::JsonError;
use thiserror::Error;
use warpgate::WarpgatePluginError;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoSetupError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Install(#[from] Box<ProtoInstallError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Json(#[from] Box<JsonError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Link(#[from] Box<ProtoLinkError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Locate(#[from] Box<ProtoLocateError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Resolve(#[from] Box<ProtoResolveError>),
}

impl From<ProtoConfigError> for ProtoSetupError {
    fn from(e: ProtoConfigError) -> ProtoSetupError {
        ProtoSetupError::Config(Box::new(e))
    }
}

impl From<ProtoInstallError> for ProtoSetupError {
    fn from(e: ProtoInstallError) -> ProtoSetupError {
        ProtoSetupError::Install(Box::new(e))
    }
}

impl From<JsonError> for ProtoSetupError {
    fn from(e: JsonError) -> ProtoSetupError {
        ProtoSetupError::Json(Box::new(e))
    }
}
impl From<ProtoLayoutError> for ProtoSetupError {
    fn from(e: ProtoLayoutError) -> ProtoSetupError {
        ProtoSetupError::Layout(Box::new(e))
    }
}

impl From<ProtoLinkError> for ProtoSetupError {
    fn from(e: ProtoLinkError) -> ProtoSetupError {
        ProtoSetupError::Link(Box::new(e))
    }
}

impl From<ProtoLocateError> for ProtoSetupError {
    fn from(e: ProtoLocateError) -> ProtoSetupError {
        ProtoSetupError::Locate(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoSetupError {
    fn from(e: WarpgatePluginError) -> ProtoSetupError {
        ProtoSetupError::Plugin(Box::new(e))
    }
}

impl From<ProtoResolveError> for ProtoSetupError {
    fn from(e: ProtoResolveError) -> ProtoSetupError {
        ProtoSetupError::Resolve(Box::new(e))
    }
}
