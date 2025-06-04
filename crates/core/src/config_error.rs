use schematic::ConfigError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::toml::TomlError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum ProtoConfigError {
    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Schematic(#[from] Box<ConfigError>),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Toml(#[from] Box<TomlError>),

    #[diagnostic(code(proto::config::env_parse_failed))]
    #[error(
        "Failed to parse .env file {}.",
        .path.style(Style::Path),
    )]
    FailedParseEnvFile {
        path: PathBuf,
        #[source]
        error: Box<dotenvy::Error>,
    },

    #[diagnostic(code(proto::config::missing_env_file))]
    #[error(
        "The .env file {} does not exist. This was configured as {} in the config {}.",
        .path.style(Style::Path),
        .config.style(Style::File),
        .config_path.style(Style::Path),
    )]
    MissingEnvFile {
        path: PathBuf,
        config: String,
        config_path: PathBuf,
    },
}

unsafe impl Send for ProtoConfigError {}
unsafe impl Sync for ProtoConfigError {}

impl From<FsError> for ProtoConfigError {
    fn from(e: FsError) -> ProtoConfigError {
        ProtoConfigError::Fs(Box::new(e))
    }
}

impl From<ConfigError> for ProtoConfigError {
    fn from(e: ConfigError) -> ProtoConfigError {
        ProtoConfigError::Schematic(Box::new(e))
    }
}

impl From<TomlError> for ProtoConfigError {
    fn from(e: TomlError) -> ProtoConfigError {
        ProtoConfigError::Toml(Box::new(e))
    }
}
