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

    #[diagnostic(code(proto::toml))]
    #[error(transparent)]
    Toml(#[from] starbase_utils::toml::TomlError),

    #[diagnostic(code(proto::download::failed))]
    #[error("Failed to download tool from <url>{0}</url>: {1}")]
    DownloadFailed(String, String),

    #[diagnostic(code(proto::execute::missing_bin))]
    #[error("Unable to find an executable binary for {0}, expected file <path>{1}</path> does not exist.")]
    ExecuteMissingBin(String, PathBuf),

    #[diagnostic(code(proto::http))]
    #[error("Failure for <url>{url}</url>")]
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
    #[error("Invalid configuration for <path>{0}</path>: {1}")]
    InvalidConfig(PathBuf, String),

    #[diagnostic(code(proto::misc))]
    #[error("{0}")]
    Message(String),

    #[diagnostic(code(proto::config::missing))]
    #[error("Could not locate a <file>{0}</file> configuration file.")]
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
    #[error("Invalid version {version}")]
    Semver {
        version: String,
        #[source]
        error: semver::Error,
    },

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version {version}")]
    SemverLenient {
        version: String,
        #[source]
        error: lenient_semver::parser::OwnedError,
    },

    #[diagnostic(code(proto::shim::failed))]
    #[error("Failed to create shim")]
    Shim(#[source] tinytemplate::error::Error),

    #[diagnostic(code(proto::unsupported::archive))]
    #[error("Unable to unpack <path>{0}</path>, unsupported archive format {1}.")]
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
    #[error(
        "Checksum has failed for <path>{0}</path>, which was verified using <path>{1}</path>."
    )]
    VerifyInvalidChecksum(PathBuf, PathBuf),

    #[diagnostic(code(proto::alias::unknown))]
    #[error("Version alias <id>{0}</id> could not be found in the manifest.")]
    VersionUnknownAlias(String),

    #[diagnostic(code(proto::version::unresolved))]
    #[error("Failed to resolve a semantic version for {0}.")]
    VersionResolveFailed(String),

    #[diagnostic(code(proto::env::path_failed))]
    #[error("Failed to write to PATH.")]
    WritePathFailed,

    #[diagnostic(code(proto::zip::failed))]
    #[error("Failed using zip archive.")]
    Zip(#[from] zip::result::ZipError),
}
