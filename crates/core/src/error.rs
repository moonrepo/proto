use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoError {
    #[error("{0}")]
    Message(String),

    #[diagnostic(
        code(proto::download::missing),
        help = "Please refer to the tool's official documentation."
    )]
    #[error("Tool download {} does not exist. This version may not be supported for your current operating system or architecture.", .url.style(Style::Url))]
    DownloadNotFound { url: String },

    #[diagnostic(code(proto::download::failed))]
    #[error("Failed to download tool from {}: {status}", .url.style(Style::Url))]
    DownloadFailed { url: String, status: String },

    #[diagnostic(code(proto::misc::offline))]
    #[error("Internet connection required, unable to download and install tools.")]
    InternetConnectionRequired,

    #[diagnostic(code(proto::verify::invalid_checksum))]
    #[error(
        "Checksum has failed for {}, which was verified using {}.", .download.style(Style::Path), .checksum.style(Style::Path)
    )]
    InvalidChecksum {
        checksum: PathBuf,
        download: PathBuf,
    },

    #[diagnostic(code(proto::env::home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(proto::execute::missing_bin))]
    #[error("Unable to find an executable binary for {tool}, expected file {} does not exist.", .bin.style(Style::Path))]
    MissingToolBin { tool: String, bin: PathBuf },

    #[diagnostic(code(proto::tool::required))]
    #[error(
        "This project requires {tool} {}, but this version has not been installed. Install it with {}!",
        .version.style(Style::Hash),
        .command.style(Style::Shell),
    )]
    MissingToolForRun {
        tool: String,
        version: String,
        command: String,
    },

    #[diagnostic(code(proto::tool::unknown))]
    #[error(
        "{} is not a built-in tool or has not been configured as a plugin, unable to proceed.", .id.style(Style::Id)
    )]
    UnknownTool { id: String },

    #[diagnostic(code(proto::unsupported::shell))]
    #[error("Unable to detect shell.")]
    UnsupportedShell,

    #[diagnostic(
        code(proto::version::undetected),
        help = "Has the tool been installed?"
    )]
    #[error("Failed to detect an applicable version to run {} with. Try pinning a local or global version, or passing the version as an argument.", .tool.style(Style::Id))]
    VersionDetectFailed { tool: String },

    #[diagnostic(code(proto::version::unresolved))]
    #[error("Failed to resolve a semantic version for {}.", .version.style(Style::Hash))]
    VersionResolveFailed { version: String },

    #[diagnostic(code(proto::http))]
    #[error("Failed to request {}.", .url.style(Style::Url))]
    Http {
        url: String,
        #[source]
        error: reqwest::Error,
    },

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version or requirement {}.", .version.style(Style::Hash))]
    Semver {
        version: String,
        #[source]
        error: semver::Error,
    },

    #[diagnostic(code(proto::shim::failed))]
    #[error("Failed to create shim {}.", .path.style(Style::Path))]
    Shim {
        path: PathBuf,
        #[source]
        error: tinytemplate::error::Error,
    },
}
