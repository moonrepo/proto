use crate::config_error::ProtoConfigError;
use crate::flow::install::ProtoInstallError;
use crate::flow::link::ProtoLinkError;
use crate::flow::locate::ProtoLocateError;
use crate::flow::lock::ProtoLockError;
use crate::flow::resolve::ProtoResolveError;
use crate::layout::ProtoLayoutError;
use starbase_utils::json::JsonError;
use thiserror::Error;
use warpgate::WarpgatePluginError;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoManageError {
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
    Lock(#[from] Box<ProtoLockError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Resolve(#[from] Box<ProtoResolveError>),
}

impl From<ProtoConfigError> for ProtoManageError {
    fn from(e: ProtoConfigError) -> ProtoManageError {
        ProtoManageError::Config(Box::new(e))
    }
}

impl From<ProtoInstallError> for ProtoManageError {
    fn from(e: ProtoInstallError) -> ProtoManageError {
        ProtoManageError::Install(Box::new(e))
    }
}

impl From<JsonError> for ProtoManageError {
    fn from(e: JsonError) -> ProtoManageError {
        ProtoManageError::Json(Box::new(e))
    }
}

impl From<ProtoLayoutError> for ProtoManageError {
    fn from(e: ProtoLayoutError) -> ProtoManageError {
        ProtoManageError::Layout(Box::new(e))
    }
}

impl From<ProtoLinkError> for ProtoManageError {
    fn from(e: ProtoLinkError) -> ProtoManageError {
        ProtoManageError::Link(Box::new(e))
    }
}

impl From<ProtoLocateError> for ProtoManageError {
    fn from(e: ProtoLocateError) -> ProtoManageError {
        ProtoManageError::Locate(Box::new(e))
    }
}

impl From<ProtoLockError> for ProtoManageError {
    fn from(e: ProtoLockError) -> ProtoManageError {
        ProtoManageError::Lock(Box::new(e))
    }
}

impl From<WarpgatePluginError> for ProtoManageError {
    fn from(e: WarpgatePluginError) -> ProtoManageError {
        ProtoManageError::Plugin(Box::new(e))
    }
}

impl From<ProtoResolveError> for ProtoManageError {
    fn from(e: ProtoResolveError) -> ProtoManageError {
        ProtoManageError::Resolve(Box::new(e))
    }
}
