use schematic::ConfigError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::toml::TomlError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum ProtoConfigError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Schematic(#[from] Box<ConfigError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Toml(#[from] Box<TomlError>),

    #[cfg_attr(feature = "miette", diagnostic(code(proto::config::env_parse_failed)))]
    #[error(
        "Failed to parse .env file {}.",
        .path.style(Style::Path),
    )]
    FailedParseEnvFile {
        path: PathBuf,
        #[source]
        error: Box<dotenvy::Error>,
    },

    #[cfg_attr(feature = "miette", diagnostic(code(proto::config::missing_env_file)))]
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
