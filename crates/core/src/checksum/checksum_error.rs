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
}
