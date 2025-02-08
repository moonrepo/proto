use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoConfigError {
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
