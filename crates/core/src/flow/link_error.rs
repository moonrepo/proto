use super::locate_error::ProtoLocateError;
use crate::layout::ProtoLayoutError;
use crate::tool_error::ProtoToolError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoLinkError {
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[error(transparent)]
    Layout(#[from] Box<ProtoLayoutError>),

    #[error(transparent)]
    Locate(#[from] Box<ProtoLocateError>),

    #[error(transparent)]
    Tool(#[from] Box<ProtoToolError>),

    #[cfg_attr(feature = "miette", diagnostic(code(proto::link::failed_args_parse)))]
    #[error("Failed to parse shim arguments string {}.", .args.style(Style::Shell))]
    FailedArgsParse {
        args: String,
        #[source]
        error: Box<shell_words::ParseError>,
    },
}

impl From<FsError> for ProtoLinkError {
    fn from(e: FsError) -> ProtoLinkError {
        ProtoLinkError::Fs(Box::new(e))
    }
}

impl From<ProtoLayoutError> for ProtoLinkError {
    fn from(e: ProtoLayoutError) -> ProtoLinkError {
        ProtoLinkError::Layout(Box::new(e))
    }
}

impl From<ProtoLocateError> for ProtoLinkError {
    fn from(e: ProtoLocateError) -> ProtoLinkError {
        ProtoLinkError::Locate(Box::new(e))
    }
}

impl From<ProtoToolError> for ProtoLinkError {
    fn from(e: ProtoToolError) -> ProtoLinkError {
        ProtoLinkError::Tool(Box::new(e))
    }
}
