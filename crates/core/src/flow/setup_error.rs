use crate::config_error::ProtoConfigError;
use crate::flow::install::ProtoInstallError;
use crate::flow::link::ProtoLinkError;
use crate::flow::locate::ProtoLocateError;
use crate::flow::resolve::ProtoResolveError;
use crate::layout::ProtoLayoutError;
use crate::tool_error::ProtoToolError;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoSetupError {
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[error(transparent)]
    Install(#[from] Box<ProtoInstallError>),

    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[error(transparent)]
    Link(#[from] Box<ProtoLinkError>),

    #[error(transparent)]
    Locate(#[from] Box<ProtoLocateError>),

    #[error(transparent)]
    Resolve(#[from] Box<ProtoResolveError>),

    #[error(transparent)]
    Tool(#[from] Box<ProtoToolError>),
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

impl From<ProtoResolveError> for ProtoSetupError {
    fn from(e: ProtoResolveError) -> ProtoSetupError {
        ProtoSetupError::Resolve(Box::new(e))
    }
}

impl From<ProtoToolError> for ProtoSetupError {
    fn from(e: ProtoToolError) -> ProtoSetupError {
        ProtoSetupError::Tool(Box::new(e))
    }
}
