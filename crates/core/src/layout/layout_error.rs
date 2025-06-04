use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::json::JsonError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoLayoutError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Json(#[from] Box<JsonError>),

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::store::shim::create_failed))
    )]
    #[error("Failed to create shim {}.", .path.style(Style::Path))]
    FailedCreateShim {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(proto::store::shim::missing_binary))
    )]
    #[error(
        "Unable to create shims as the {} binary cannot be found.\nLooked in the {} environment variable and {} directory.",
        "proto-shim".style(Style::Id),
        "PROTO_HOME".style(Style::Property),
        .bin_dir.style(Style::Path),
    )]
    MissingShimBinary { bin_dir: PathBuf },
}

impl From<FsError> for ProtoLayoutError {
    fn from(e: FsError) -> ProtoLayoutError {
        ProtoLayoutError::Fs(Box::new(e))
    }
}

impl From<JsonError> for ProtoLayoutError {
    fn from(e: JsonError) -> ProtoLayoutError {
        ProtoLayoutError::Json(Box::new(e))
    }
}
