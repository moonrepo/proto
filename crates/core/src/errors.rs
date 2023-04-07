use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoError {
    #[diagnostic(code(proto::fs))]
    #[error(transparent)]
    Fs(#[from] starbase_utils::fs::FsError),

    #[diagnostic(code(proto::json))]
    #[error(transparent)]
    Json(#[from] starbase_utils::json::JsonError),

    #[diagnostic(code(proto::download::failed))]
    #[error("Failed to download tool from {0}: {1}")]
    DownloadFailed(String, String),

    #[diagnostic(code(proto::execute::missing_bin))]
    #[error("Unable to find an executable binary for {0}, expected file {1} does not exist.")]
    ExecuteMissingBin(String, PathBuf),

    #[diagnostic(code(proto::fs))]
    #[error("HTTP failure for {url}: {error}")]
    Http {
        url: String,
        #[source]
        error: reqwest::Error,
    },

    #[diagnostic(code(proto::download::file_missing))]
    #[error("Unable to install {0}, download file is missing.")]
    InstallMissingDownload(String),

    #[diagnostic(code(proto::misc::offline))]
    #[error("Internet connection required, unable to download and install tools.")]
    InternetConnectionRequired,

    #[diagnostic(code(proto::config::invalid))]
    #[error("Invalid configuration for {0}: {1}")]
    InvalidConfig(PathBuf, String),

    #[diagnostic(code(proto::misc))]
    #[error("{0}")]
    Message(String),

    #[diagnostic(code(proto::config::missing))]
    #[error("Could not locate a {0} configuration file.")]
    MissingConfig(String),

    #[diagnostic(code(proto::env::home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(proto::env::path))]
    #[error("Unable to determine PATH.")]
    MissingPathEnv,

    #[diagnostic(code(proto::tool::missing))]
    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),

    #[diagnostic(code(proto::tool::missing_run))]
    #[error(
        "This project requires {0} {1}, but this version has not been installed. Install it with {2}!"
    )]
    MissingToolForRun(String, String, String),

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version {version}: {error}")]
    Semver {
        version: String,
        #[source]
        error: semver::Error,
    },

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version {version}: {error}")]
    SemverLenient {
        version: String,
        #[source]
        error: lenient_semver::parser::Error<'static>,
    },

    #[diagnostic(code(proto::shim::failed))]
    #[error("Failed shim: {0}")]
    Shim(#[source] tinytemplate::error::Error),

    #[diagnostic(code(proto::toml::parse))]
    #[error("Failed to parse TOML file {path}: {error}")]
    Toml {
        path: PathBuf,
        error: toml::de::Error,
    },

    #[diagnostic(code(proto::toml::stringify))]
    #[error("Failed to stringify TOML file {path}: {error}")]
    TomlStringify {
        path: PathBuf,
        error: toml::ser::Error,
    },

    #[diagnostic(code(proto::unsupported::archive))]
    #[error("Unable to unpack {0}, unsupported archive format {1}.")]
    UnsupportedArchiveFormat(PathBuf, String),

    #[diagnostic(code(proto::unsupported::arch))]
    #[error("Unable to install {0}, unsupported architecture {1}.")]
    UnsupportedArchitecture(String, String),

    #[diagnostic(code(proto::unsupported::platform))]
    #[error("Unable to install {0}, unsupported platform {1}.")]
    UnsupportedPlatform(String, String),

    #[diagnostic(code(proto::unsupported::shell))]
    #[error("Unable to detect shell.")]
    UnsupportedShell,

    #[diagnostic(code(proto::tool::unknown))]
    #[error("Tool {0} is unknown or unsupported.")]
    UnsupportedTool(String),

    #[diagnostic(code(proto::verify::invalid_checksum))]
    #[error("Checksum has failed for {0}, which was verified using {1}.")]
    VerifyInvalidChecksum(PathBuf, PathBuf),

    #[diagnostic(code(proto::alias::unknown))]
    #[error("Version alias \"{0}\" could not be found in the manifest.")]
    VersionUnknownAlias(String),

    #[diagnostic(code(proto::version::failed))]
    #[error("Failed to parse version {0}. {1}")]
    VersionParseFailed(String, String),

    #[diagnostic(code(proto::version::unresolved))]
    #[error("Failed to resolve a semantic version for {0}.")]
    VersionResolveFailed(String),

    #[diagnostic(code(proto::env::path_failed))]
    #[error("Failed to write to PATH.")]
    WritePathFailed,

    #[diagnostic(code(proto::zip::failed))]
    #[error("Failed zip archive. {0}")]
    Zip(#[from] zip::result::ZipError),
}
