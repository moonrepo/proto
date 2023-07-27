use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoError {
    #[diagnostic(
        code(proto::download::missing),
        help = "Please refer to the tool's official documentation."
    )]
    #[error("Tool download {} does not exist. This version may not be supported for your current operating system or architecture.", .0.style(Style::Url))]
    DownloadNotFound(String),

    #[diagnostic(code(proto::download::failed))]
    #[error("Failed to download tool from {}: {1}", .0.style(Style::Url))]
    DownloadFailed(String, String),

    #[diagnostic(code(proto::execute::missing_bin))]
    #[error("Unable to find an executable binary for {0}, expected file {} does not exist.", .1.style(Style::Path))]
    ExecuteMissingBin(String, PathBuf),

    #[diagnostic(code(proto::http))]
    #[error("Failure for {}", .url.style(Style::Url))]
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
    #[error("Invalid configuration for {}: {1}", .0.style(Style::Path))]
    InvalidConfig(PathBuf, String),

    #[diagnostic(code(proto::plugin::invalid_protocol))]
    #[error("Invalid plugin protocol {}", .0.style(Style::Label))]
    InvalidPluginProtocol(String),

    #[diagnostic(code(proto::plugin::invalid_locator))]
    #[error("Invalid plugin locator, must be a relative file path or an HTTPS URL.")]
    InvalidPluginLocator,

    #[diagnostic(code(proto::plugin::invalid_ext))]
    #[error("Invalid plugin locator, must have a {0} extension.")]
    InvalidPluginLocatorExt(String),

    #[diagnostic(code(proto::misc))]
    #[error("{0}")]
    Message(String),

    #[diagnostic(code(proto::config::missing))]
    #[error("Could not locate a {} configuration file.", .0.style(Style::File))]
    MissingConfig(String),

    #[diagnostic(code(proto::env::home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(proto::env::path))]
    #[error("Unable to determine PATH.")]
    MissingPathEnv,

    #[diagnostic(code(proto::plugin::missing))]
    #[error(
        "{0} is not a built-in tool and has not been configured as a plugin, unable to proceed."
    )]
    MissingPlugin(String),

    #[diagnostic(code(proto::tool::missing))]
    #[error("{0} has not been configured or installed, unable to proceed.")]
    MissingTool(String),

    #[diagnostic(code(proto::tool::required))]
    #[error(
        "This project requires {0} {1}, but this version has not been installed. Install it with {2}!"
    )]
    MissingToolForRun(String, String, String),

    #[diagnostic(code(proto::plugin::load_wasm_failed))]
    #[error("Failed to load WASM plugin. {0}")]
    PluginWasmCreateFailed(String),

    #[diagnostic(code(proto::plugin::call_wasm_failed))]
    #[error("Failed to call WASM plugin function. {0}")]
    PluginWasmCallFailed(String),

    #[diagnostic(code(proto::plugin::missing_file))]
    #[error("Plugin file {} does not exist.", .0.style(Style::Path))]
    PluginFileMissing(PathBuf),

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
    #[error("Unable to unpack {}, unsupported archive format {1}.", .0.style(Style::Path))]
    UnsupportedArchiveFormat(PathBuf, String),

    #[diagnostic(code(proto::unsupported::arch))]
    #[error("Unable to install {0}, unsupported architecture {1}.")]
    UnsupportedArchitecture(String, String),

    #[diagnostic(code(proto::unsupported::globals))]
    #[error("{0} does not support global binaries.")]
    UnsupportedGlobals(String),

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
        "Checksum has failed for {}, which was verified using {}.", .0.style(Style::Path), .1.style(Style::Path)
    )]
    VerifyInvalidChecksum(PathBuf, PathBuf),

    #[diagnostic(code(proto::alias::unknown))]
    #[error("Version alias {} could not be found in the manifest.", .0.style(Style::Id))]
    VersionUnknownAlias(String),

    #[diagnostic(code(proto::version::unresolved))]
    #[error("Failed to resolve a semantic version for {0}.")]
    VersionResolveFailed(String),

    #[diagnostic(
        code(proto::version::undetected),
        help = "Has the tool been installed?"
    )]
    #[error("Failed to detect an applicable version to run {} with. Try pinning a local or global version, or passing the version as an argument.", .0.style(Style::Shell))]
    VersionDetectFailed(String),

    #[diagnostic(code(proto::env::path_failed))]
    #[error("Failed to write to PATH.")]
    WritePathFailed,

    #[diagnostic(transparent)]
    #[error(transparent)]
    Fs(#[from] starbase_utils::fs::FsError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Json(#[from] starbase_utils::json::JsonError),

    #[diagnostic(transparent)]
    #[error(transparent)]
    Toml(#[from] starbase_utils::toml::TomlError),
}
