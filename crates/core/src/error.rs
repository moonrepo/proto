use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;
use warpgate::Id;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoError {
    #[error("{0}")]
    Message(String),

    #[diagnostic(code(proto::tool::invalid_dir))]
    #[error("{tool} inventory directory has been overridden but is not an absolute path. Only absolute paths are supported.")]
    AbsoluteInventoryDir { tool: String },

    #[diagnostic(code(proto::tool::install_failed))]
    #[error("Failed to install {tool}. {error}")]
    InstallFailed { tool: String, error: String },

    #[diagnostic(code(proto::tool::build_failed))]
    #[error("Failed to build {tool} from {}: {status}", .url.style(Style::Url))]
    BuildFailed {
        tool: String,
        url: String,
        status: String,
    },

    #[diagnostic(code(proto::misc::offline))]
    #[error("Internet connection required, unable to download, install, or run tools.")]
    InternetConnectionRequired,

    #[diagnostic(code(proto::misc::offline_version_required))]
    #[error(
        "Internet connection required to load and resolve a valid version. To work around this:\n - Pass a semantic version explicitly: {}\n - Execute the non-shim binaries instead: {}",
        .command.style(Style::Shell),
        .bin_dir.style(Style::Path)
    )]
    InternetConnectionRequiredForVersion { command: String, bin_dir: PathBuf },

    #[diagnostic(code(proto::verify::missing_public_key))]
    #[error(
        "A {} is required to verify this tool.", "checksum_public_key".style(Style::Property)
    )]
    MissingChecksumPublicKey,

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

    #[diagnostic(code(proto::shim::missing_binary))]
    #[error(
        "Unable to create shims as the {} binary cannot be found.\nLooked in the {} environment variable and {} directory.",
        "proto-shim".style(Style::Id),
        "PROTO_HOME".style(Style::Property),
        .bin_dir.style(Style::Path),
    )]
    MissingShimBinary { bin_dir: PathBuf },

    #[diagnostic(code(proto::execute::missing_file))]
    #[error("Unable to find an executable for {tool}, expected file {} does not exist.", .path.style(Style::Path))]
    MissingToolExecutable { tool: String, path: PathBuf },

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

    #[diagnostic(code(proto::tool::required))]
    #[error(
        "This project requires {tool} {} (detected from {}), but this version has not been installed. Install it with {}!",
        .version.style(Style::Hash),
        .path.style(Style::Path),
        .command.style(Style::Shell),
    )]
    MissingToolForRunWithSource {
        tool: String,
        version: String,
        command: String,
        path: PathBuf,
    },

    #[diagnostic(code(proto::tool::uninstall_failed))]
    #[error("Failed to uninstall {tool}. {error}")]
    UninstallFailed { tool: String, error: String },

    #[diagnostic(code(proto::tool::unknown))]
    #[error(
        "{} is not a built-in tool or has not been configured as a plugin, unable to proceed.", .id.style(Style::Id)
    )]
    UnknownTool { id: Id },

    #[diagnostic(code(proto::build::unsupported))]
    #[error("Build from source is not supported for {tool}.")]
    UnsupportedBuildFromSource { tool: String },

    #[diagnostic(
        code(proto::version::undetected),
        help = "Has the tool been installed?"
    )]
    #[error("Failed to detect an applicable version to run {tool} with. Try pinning a version or passing the version as an argument.")]
    VersionDetectFailed { tool: String },

    #[diagnostic(
        code(proto::version::unresolved),
        help = "Does this version exist and has it been released?"
    )]
    #[error(
        "Failed to resolve {} to a valid supported version for {tool}.",
        .version.style(Style::Hash),
    )]
    VersionResolveFailed { tool: String, version: String },

    #[diagnostic(code(proto::http))]
    #[error("Failed to request {}.", .url.style(Style::Url))]
    Http {
        url: String,
        #[source]
        error: reqwest::Error,
    },

    #[diagnostic(code(proto::verify::minisign))]
    #[error("Failed to verify minisign checksum.")]
    Minisign {
        #[source]
        error: minisign_verify::Error,
    },

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version or requirement {}.", .version.style(Style::Hash))]
    Semver {
        version: String,
        #[source]
        error: semver::Error,
    },

    #[diagnostic(code(proto::shim::create_failed))]
    #[error("Failed to create shim {}.", .path.style(Style::Path))]
    CreateShimFailed {
        path: PathBuf,
        #[source]
        error: std::io::Error,
    },
}
