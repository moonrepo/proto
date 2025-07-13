use crate::config_error::ProtoConfigError;
use starbase_styles::{Style, Stylize};
use starbase_utils::toml::TomlError;
use thiserror::Error;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoLockError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Config(#[from] Box<ProtoConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Toml(#[from] Box<TomlError>),

    #[diagnostic(
        code(proto::install::mismatched_checksum),
        help = "Is this install legitimate?"
    )]
    #[error(
        "Checksum mismatch! Received {} but expected {}.",
        .checksum.style(Style::Hash),
        .lockfile_checksum.style(Style::Hash),
    )]
    MismatchedChecksum {
        checksum: String,
        lockfile_checksum: String,
    },

    #[diagnostic(
        code(proto::install::mismatched_checksum),
        help = "Is this install legitimate?"
    )]
    #[error(
        "Checksum mismatch for {}! Received {} but expected {}.",
        .source_url.style(Style::Url),
        .checksum.style(Style::Hash),
        .lockfile_checksum.style(Style::Hash),
    )]
    MismatchedChecksumWithSource {
        checksum: String,
        lockfile_checksum: String,
        source_url: String,
    },
}

impl From<ProtoConfigError> for ProtoLockError {
    fn from(e: ProtoConfigError) -> ProtoLockError {
        ProtoLockError::Config(Box::new(e))
    }
}

impl From<TomlError> for ProtoLockError {
    fn from(e: TomlError) -> ProtoLockError {
        ProtoLockError::Toml(Box::new(e))
    }
}
