use super::locate_error::ProtoLocateError;
use crate::layout::ProtoLayoutError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::json::JsonError;
use thiserror::Error;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoLinkError {
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
    Locate(#[from] Box<ProtoLocateError>),

    #[diagnostic(code(proto::link::failed_args_parse))]
    #[error("Failed to parse shim arguments string {}.", .args.style(Style::Shell))]
    FailedArgsParse {
        args: String,
        #[source]
        error: Box<shell_words::ParseError>,
    },
}

unsafe impl Send for ProtoLinkError {}
unsafe impl Sync for ProtoLinkError {}

impl From<FsError> for ProtoLinkError {
    fn from(e: FsError) -> ProtoLinkError {
        ProtoLinkError::Fs(Box::new(e))
    }
}

impl From<JsonError> for ProtoLinkError {
    fn from(e: JsonError) -> ProtoLinkError {
        ProtoLinkError::Json(Box::new(e))
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
