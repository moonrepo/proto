use crate::proto_config::PROTO_CONFIG_NAME;
use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;
use warpgate::Id;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoErrorOld {
    #[error("{0}")]
    Message(String),

    #[diagnostic(code(proto::tool::invalid_dir))]
    #[error("{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.", .dir.style(Style::Path))]
    AbsoluteInventoryDir { tool: String, dir: PathBuf },

    #[diagnostic(code(proto::tool::build_failed))]
    #[error("Failed to build {tool} from {}: {status}", .url.style(Style::Url))]
    BuildFailed {
        tool: String,
        url: String,
        status: String,
    },

    #[diagnostic(code(proto::offline))]
    #[error("Internet connection required, unable to download, install, or run tools.")]
    InternetConnectionRequired,

    #[diagnostic(code(proto::minimum_version_requirement))]
    #[error(
        "Unable to use the {tool} plugin with identifier {}, as it requires a minimum proto version of {}, but found {} instead.",
        .id.to_string().style(Style::Id),
        .expected.style(Style::Hash),
        .actual.style(Style::Hash)
    )]
    InvalidMinimumVersion {
        tool: String,
        id: Id,
        expected: String,
        actual: String,
    },

    #[diagnostic(code(proto::env::home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(proto::tool::required))]
    #[error(
        "This project requires {tool} {}, but this version has not been installed. Install it with {}, or enable the {} setting to automatically install missing versions!",
        .version.style(Style::Hash),
        .command.style(Style::Shell),
        "auto-install".style(Style::Property),
    )]
    MissingToolForRun {
        tool: String,
        version: String,
        command: String,
    },

    #[diagnostic(code(proto::tool::required))]
    #[error(
        "This project requires {tool} {} (detected from {}), but this version has not been installed. Install it with {}, or enable the {} setting to automatically install missing versions!",
        .version.style(Style::Hash),
        .path.style(Style::Path),
        .command.style(Style::Shell),
        "auto-install".style(Style::Property),
    )]
    MissingToolForRunWithSource {
        tool: String,
        version: String,
        command: String,
        path: PathBuf,
    },

    #[diagnostic(code(proto::tool::unknown))]
    #[error(
        "Unable to proceed, {} is not a built-in tool and has not been configured with {} in a {} file.\n\nLearn more about plugins: {}\nSearch community plugins: {}",
        .id.to_string().style(Style::Id),
        "[plugins]".style(Style::Property),
        PROTO_CONFIG_NAME.style(Style::File),
        "https://moonrepo.dev/docs/proto/plugins".style(Style::Url),
        format!("proto plugin search {}", .id).style(Style::Shell),
    )]
    UnknownTool { id: Id },

    #[diagnostic(
        code(proto::version::undetected),
        help = "Has the tool been installed?"
    )]
    #[error(
        "Failed to detect an applicable version to run {tool} with. Try pinning a version with {} or passing the version as an argument.",
        "proto pin".style(Style::Shell),
    )]
    VersionDetectFailed { tool: String },

    #[diagnostic(code(proto::http))]
    #[error("Failed to request {}.", .url.style(Style::Url))]
    Http {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version or requirement {}.", .version.style(Style::Hash))]
    VersionSpec {
        version: String,
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[diagnostic(code(proto::env::missing_file))]
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

    #[diagnostic(code(proto::env::parse_failed))]
    #[error(
        "Failed to parse .env file {}.",
        .path.style(Style::Path),
    )]
    EnvFileParseFailed {
        path: PathBuf,
        #[source]
        error: Box<dotenvy::Error>,
    },
}
