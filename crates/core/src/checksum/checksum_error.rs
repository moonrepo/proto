use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoChecksumError {
    #[diagnostic(code(proto::checksum::minisign))]
    #[error("Failed to verify minisign checksum.")]
    Minisign {
        #[source]
        error: Box<minisign_verify::Error>,
    },

    #[diagnostic(code(proto::checksum::missing_public_key))]
    #[error(
        "A {} is required to verify this tool. This setting must be implemented in the plugin.", "checksum_public_key".style(Style::Property)
    )]
    MissingPublicKey,

    #[diagnostic(
        code(proto::checksum::unknown_type),
        help = "Try using a more explicit file extension."
    )]
    #[error(
        "Unknown checksum type. Unable to derive the type from {}.",
        .file.style(Style::File)
    )]
    UnknownType { file: String },

    #[diagnostic(
        code(proto::checksum::unsupported_algorithm),
        help = "Try using a more explicit file extension."
    )]
    #[error(
        "Unsupported checksum algorithm {}.",
        .algo.style(Style::Symbol)
    )]
    UnsupportedAlgorithm { algo: String },
}
