use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoChecksumError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(code(proto::checksum::minisign))]
    #[error("Failed to verify minisign checksum.")]
    Minisign {
        #[source]
        error: Box<minisign_verify::Error>,
    },

    #[diagnostic(code(proto::checksum::sha))]
    #[error("Failed to verify SHA checksum.")]
    Sha {
        #[source]
        error: Box<FsError>,
    },

    #[diagnostic(code(proto::checksum::missing_public_key))]
    #[error(
        "A {} is required to verify this tool. This setting must be implemented in the plugin.", "checksum_public_key".style(Style::Property)
    )]
    MissingPublicKey,

    #[diagnostic(
        code(proto::checksum::unknown_algorithm),
        help = "Try using a more explicit file extension."
    )]
    #[error(
        "Unknown checksum algorithm. Unable to derive from {}.",
        .path.style(Style::Path)
    )]
    UnknownAlgorithm { path: PathBuf },

    #[diagnostic(code(proto::checksum::unsupported_algorithm))]
    #[error(
        "Unsupported checksum algorithm {}.",
        .algo.style(Style::Symbol)
    )]
    UnsupportedAlgorithm { algo: String },
}

impl From<FsError> for ProtoChecksumError {
    fn from(e: FsError) -> ProtoChecksumError {
        ProtoChecksumError::Fs(Box::new(e))
    }
}
