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
    MissingChecksumPublicKey,

    #[diagnostic(code(proto::checksum::missing_type))]
    #[error("Checksum type is not defined.")]
    MissingChecksumType,

    #[diagnostic(code(proto::checksum::unknown_type))]
    #[error(
        "Unknown or unsupported checksum type {}.", .kind.style(Style::Property)
    )]
    UnknownChecksumType { kind: String },
}
