use crate::proto_config::PROTO_CONFIG_NAME;
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
    #[error("{tool} inventory directory has been overridden with {} but it's not an absolute path. Only absolute paths are supported.", .dir.style(Style::Path))]
    AbsoluteInventoryDir { tool: String, dir: PathBuf },

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

    #[diagnostic(code(proto::tool::uninstall_failed))]
    #[error("Failed to uninstall {tool}. {error}")]
    UninstallFailed { tool: String, error: String },

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

    #[diagnostic(code(proto::prebuilt::unsupported))]
    #[error("Downloading a pre-built is not supported for {tool}. Try building from source by passing {}.", "--build".style(Style::Shell))]
    UnsupportedDownloadPrebuilt { tool: String },

    #[diagnostic(code(proto::build::unsupported))]
    #[error("Building from source is not supported for {tool}. Try downloading a pre-built by passing {}.", "--no-build".style(Style::Shell))]
    UnsupportedBuildFromSource { tool: String },

    #[diagnostic(
        code(proto::version::undetected),
        help = "Has the tool been installed?"
    )]
    #[error(
        "Failed to detect an applicable version to run {tool} with. Try pinning a version with {} or passing the version as an argument.",
        "proto pin".style(Style::Shell),
    )]
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
        error: Box<reqwest::Error>,
    },

    #[diagnostic(code(proto::verify::minisign))]
    #[error("Failed to verify minisign checksum.")]
    Minisign {
        #[source]
        error: Box<minisign_verify::Error>,
    },

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version or requirement {}.", .version.style(Style::Hash))]
    VersionSpec {
        version: String,
        #[source]
        error: Box<version_spec::SpecError>,
    },

    #[diagnostic(code(proto::shim::create_failed))]
    #[error("Failed to create shim {}.", .path.style(Style::Path))]
    CreateShimFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
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
