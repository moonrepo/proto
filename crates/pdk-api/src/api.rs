use crate::host::{HostArch, HostOS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub use semver::{Version, VersionReq};

/// Represents an empty input.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EmptyInput {}

/// Information about the host environment (the current runtime).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Environment {
    /// Current architecture.
    pub arch: HostArch,
    /// Current operating system.
    pub os: HostOS,
    /// Requested environment variables. Only non-empty values are included.
    pub vars: HashMap<String, String>,
    /// Current resolved version. Will be empty if not resolved.
    pub version: String,
}

/// Supported types of plugins.
#[derive(Debug, Default, Deserialize, Serialize)]
pub enum PluginType {
    #[default]
    Language,
    DependencyManager,
    CLI,
}

/// Input passed to the `register_tool` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ToolMetadataInput {
    /// ID of the tool, as it was configured.
    pub id: String,
    /// Current environment.
    pub env: Environment,
}

/// Output returned by the `register_tool` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ToolMetadataOutput {
    /// Environment variables that should be extracted
    /// and passed to other function call inputs.
    pub env_vars: Vec<String>,
    /// Human readable name of the tool.
    pub name: String,
    /// Type of the tool.
    pub type_of: PluginType,
}

// Detector

/// Output returned by the `detect_version_files` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DetectVersionOutput {
    /// List of files that should be checked for version information.
    pub files: Vec<String>,
}

/// Input passed to the `parse_version_file` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ParseVersionInput {
    /// File contents to parse/extract a version from.
    pub content: String,
    /// Current environment.
    pub env: Environment,
    /// Name of file that's being parsed.
    pub file: String,
}

/// Output returned by the `parse_version_file` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ParseVersionOutput {
    /// The version that was extracted from the file.
    /// Can be a semantic version or a version requirement/range.
    pub version: Option<String>,
}

// Downloader, Installer, Verifier

/// Input passed to the `download_prebuilt` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DownloadPrebuiltInput {
    /// Current environment.
    pub env: Environment,
}

/// Output returned by the `download_prebuilt` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DownloadPrebuiltOutput {
    /// Name of the direct folder within the archive that contains the tool,
    /// and will be removed when unpacking the archive.
    pub archive_prefix: Option<String>,
    /// Relative path from the installation directory to the binary.
    /// If not provided, will use the tool `id` as the binary name.
    pub bin_path: Option<String>,
    /// File name of the checksum to download. If not provided,
    /// will attempt to extract it from the URL.
    pub checksum_name: Option<String>,
    /// A secure URL to download the checksum file for verification.
    /// If the tool does not support checksum verification, this setting can be omitted.
    pub checksum_url: Option<String>,
    /// File name of the archive to download. If not provided,
    /// will attempt to extract it from the URL.
    pub download_name: Option<String>,
    /// A secure URL to download the tool/archive.
    pub download_url: String,
}

/// Input passed to the `unpack_archive` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct UnpackArchiveInput {
    /// Virtual path to the downloaded file.
    pub input_file: PathBuf,
    /// Current environment.
    pub env: Environment,
    /// Virtual directory to unpack the archive into, or copy the binary to.
    pub output_dir: PathBuf,
}

/// Output returned by the `verify_checksum` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VerifyChecksumInput {
    /// Virtual path to the checksum file.
    pub checksum_file: PathBuf,
    /// Virtual path to the downloaded file.
    pub download_file: PathBuf,
    /// Current environment.
    pub env: Environment,
}

/// Output returned by the `verify_checksum` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct VerifyChecksumOutput {
    pub verified: bool,
}

// Executor

/// Input passed to the `locate_bins` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct LocateBinsInput {
    /// Current environment.
    pub env: Environment,
    /// Virtual path to the tool's installation directory.
    pub tool_dir: PathBuf,
}

/// Output returned by the `locate_bins` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct LocateBinsOutput {
    /// Relative path from the tool directory to the binary to execute.
    pub bin_path: Option<String>,
    /// List of directory paths to find the globals installation directory.
    /// Each path supports environment variable expansion.
    pub globals_lookup_dirs: Vec<String>,
}

// Resolver

/// Input passed to the `load_versions` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct LoadVersionsInput {
    /// Current environment.
    pub env: Environment,
}

/// Output returned by the `load_versions` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct LoadVersionsOutput {
    /// Latest stable version.
    pub latest: Option<Version>,
    /// Mapping of aliases (channels, etc) to a version.
    pub aliases: HashMap<String, Version>,
    /// List of available production versions to install.
    pub versions: Vec<Version>,
    /// List of available canary versions to install.
    pub canary_versions: Vec<Version>,
}

/// Input passed to the `resolve_version` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ResolveVersionInput {
    /// Current resolved version candidate. Will be used if no replacement version is provided.
    // pub candidate: String,
    /// The alias or version currently being resolved.
    pub initial: String,
    /// Current environment.
    pub env: Environment,
}

/// Output returned by the `resolve_version` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ResolveVersionOutput {
    /// New alias or version candidate.
    pub candidate: Option<String>,
}

// Shimmer

/// Configuration for individual shim files.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ShimConfig {
    /// Relative path from the tool directory to the binary to execute.
    pub bin_path: Option<String>,
    /// Name of a parent binary that's required for this shim to work.
    /// For example, `npm` requires `node`.
    pub parent_bin: Option<String>,
    /// Custom args to prepend to user-provided args.
    pub before_args: Option<String>,
    /// Custom args to append to user-provided args.
    pub after_args: Option<String>,
}

/// Input passed to the `create_shims` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CreateShimsInput {
    /// Current environment.
    pub env: Environment,
}

/// Output returned by the `create_shims` function.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CreateShimsOutput {
    /// Configures the default/primary global shim.
    pub primary: Option<ShimConfig>,
    /// Additional global shims to create in the `~/.proto/bin` directory.
    /// Maps a shim name to a relative binary path.
    pub global_shims: HashMap<String, ShimConfig>,
    /// Local shims to create in the `~/.proto/tools/<id>/<version>/shims` directory.
    /// Maps a shim name to its configuration.
    pub local_shims: HashMap<String, ShimConfig>,
}
