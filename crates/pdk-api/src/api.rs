use crate::host::{HostArch, HostOS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

pub use semver::{Version, VersionReq};

#[macro_export]
macro_rules! json_struct {
    ($struct:item) => {
        #[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(default)]
        $struct
    };
}

#[macro_export]
macro_rules! json_enum {
    ($struct:item) => {
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        $struct
    };
}

json_struct!(
    /// Represents an empty input.
    pub struct EmptyInput {}
);

json_struct!(
    /// Information about the host environment (the current runtime).
    pub struct Environment {
        /// Current architecture.
        pub arch: HostArch,

        /// ID of the current plugin.
        pub id: String,

        /// Current operating system.
        pub os: HostOS,

        /// Requested environment variables. Only non-empty values are included.
        pub vars: HashMap<String, String>,

        /// Current resolved version. Will be empty if not resolved.
        pub version: String,
    }
);

json_enum!(
    /// Supported types of plugins.
    #[derive(Default)]
    pub enum PluginType {
        #[default]
        Language,
        DependencyManager,
        CLI,
    }
);

json_struct!(
    /// Input passed to the `register_tool` function.
    pub struct ToolMetadataInput {
        /// ID of the tool, as it was configured.
        pub id: String,

        /// Current environment.
        pub env: Environment,

        /// Virtual path to the user's home directory.
        pub home_dir: PathBuf,
    }
);

json_struct!(
    /// Controls aspects of the tool inventory.
    pub struct ToolInventoryMetadata {
        /// Disable progress bars when installing or uninstalling tools.
        pub disable_progress_bars: bool,

        /// Override the tool inventory directory (where all versions are installed).
        /// This is an advanced feature and should only be used when absolutely necessary.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub override_dir: Option<PathBuf>,

        /// Suffix to append to all versions when labeling directories.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version_suffix: Option<String>,
    }
);

json_struct!(
    /// Output returned by the `register_tool` function.
    pub struct ToolMetadataOutput {
        /// Default alias or version to use as a fallback.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_version: Option<String>,

        /// Environment variables that should be extracted
        /// and passed to other function call inputs.
        pub env_vars: Vec<String>,

        /// Controls aspects of the tool inventory.
        pub inventory: ToolInventoryMetadata,

        /// Human readable name of the tool.
        pub name: String,

        /// Type of the tool.
        pub type_of: PluginType,
    }
);

// Detector

json_struct!(
    /// Output returned by the `detect_version_files` function.
    pub struct DetectVersionOutput {
        /// List of files that should be checked for version information.
        pub files: Vec<String>,
    }
);

json_struct!(
    /// Input passed to the `parse_version_file` function.
    pub struct ParseVersionFileInput {
        /// File contents to parse/extract a version from.
        pub content: String,

        /// Current environment.
        pub env: Environment,

        /// Name of file that's being parsed.
        pub file: String,
    }
);

json_struct!(
    /// Output returned by the `parse_version_file` function.
    pub struct ParseVersionFileOutput {
        /// The version that was extracted from the file.
        /// Can be a semantic version or a version requirement/range.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<String>,
    }
);

// Downloader, Installer, Verifier

json_struct!(
    /// Input passed to the `native_install` function.
    pub struct NativeInstallInput {
        /// Current environment.
        pub env: Environment,

        /// Virtual path to the user's home directory.
        pub home_dir: PathBuf,

        /// Virtual path to the tool's installation directory.
        pub tool_dir: PathBuf,
    }
);

json_struct!(
    /// Output returned by the `native_install` function.
    pub struct NativeInstallOutput {
        /// Wheterh the install was successful.
        pub installed: bool,
    }
);

json_struct!(
    /// Input passed to the `download_prebuilt` function.
    pub struct DownloadPrebuiltInput {
        /// Current environment.
        pub env: Environment,
    }
);

json_struct!(
    /// Output returned by the `download_prebuilt` function.
    pub struct DownloadPrebuiltOutput {
        /// Name of the direct folder within the archive that contains the tool,
        /// and will be removed when unpacking the archive.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub archive_prefix: Option<String>,

        /// File name of the checksum to download. If not provided,
        /// will attempt to extract it from the URL.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub checksum_name: Option<String>,

        /// A secure URL to download the checksum file for verification.
        /// If the tool does not support checksum verification, this setting can be omitted.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub checksum_url: Option<String>,

        /// File name of the archive to download. If not provided,
        /// will attempt to extract it from the URL.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub download_name: Option<String>,

        /// A secure URL to download the tool/archive.
        pub download_url: String,
    }
);

json_struct!(
    /// Input passed to the `unpack_archive` function.
    pub struct UnpackArchiveInput {
        /// Virtual path to the downloaded file.
        pub input_file: PathBuf,

        /// Current environment.
        pub env: Environment,

        /// Virtual directory to unpack the archive into, or copy the binary to.
        pub output_dir: PathBuf,
    }
);

json_struct!(
    /// Output returned by the `verify_checksum` function.
    pub struct VerifyChecksumInput {
        /// The SHA-256 hash of the downloaded file.
        pub checksum: String,

        /// Virtual path to the checksum file.
        pub checksum_file: PathBuf,

        /// Virtual path to the downloaded file.
        pub download_file: PathBuf,

        /// Current environment.
        pub env: Environment,
    }
);

json_struct!(
    /// Output returned by the `verify_checksum` function.
    pub struct VerifyChecksumOutput {
        pub verified: bool,
    }
);

// Executor

json_struct!(
    /// Input passed to the `locate_bins` function.
    pub struct LocateBinsInput {
        /// Current environment.
        pub env: Environment,

        /// Virtual path to the user's home directory.
        pub home_dir: PathBuf,

        /// Virtual path to the tool's installation directory.
        pub tool_dir: PathBuf,
    }
);

json_struct!(
    /// Output returned by the `locate_bins` function.
    pub struct LocateBinsOutput {
        /// Relative path from the tool directory to the binary to execute.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bin_path: Option<PathBuf>,

        /// When true, the last item in `globals_lookup_dirs` will be used,
        /// regardless if it exists on the file system or not.
        pub fallback_last_globals_dir: bool,

        /// List of directory paths to find the globals installation directory.
        /// Each path supports environment variable expansion.
        pub globals_lookup_dirs: Vec<String>,

        /// A string that all global binaries are prefixed with, and will be removed
        /// when listing and filtering available globals.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub globals_prefix: Option<String>,
    }
);

// Resolver

json_struct!(
    /// Input passed to the `load_versions` function.
    pub struct LoadVersionsInput {
        /// Current environment.
        pub env: Environment,

        /// The alias or version currently being resolved.
        pub initial: String,
    }
);

json_struct!(
    /// Output returned by the `load_versions` function.
    pub struct LoadVersionsOutput {
        /// Latest stable version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latest: Option<Version>,

        /// Mapping of aliases (channels, etc) to a version.
        pub aliases: HashMap<String, Version>,

        /// List of available production versions to install.
        pub versions: Vec<Version>,

        /// List of available canary versions to install.
        pub canary_versions: Vec<Version>,
    }
);

impl LoadVersionsOutput {
    /// Create the output from a list of Git tags, as semantic versions.
    /// The latest version will be the highest version number.
    pub fn from_tags(tags: &[String]) -> anyhow::Result<Self> {
        let mut output = LoadVersionsOutput::default();
        let mut latest = Version::new(0, 0, 0);

        for tag in tags {
            let version = Version::parse(tag)?;

            if version.pre.is_empty() && version.build.is_empty() && version > latest {
                latest = version.clone();
            }

            output.versions.push(version);
        }

        output.latest = Some(latest.clone());
        output.aliases.insert("latest".into(), latest);

        Ok(output)
    }
}

json_struct!(
    /// Input passed to the `resolve_version` function.
    pub struct ResolveVersionInput {
        /// The alias or version currently being resolved.
        pub initial: String,

        /// Current environment.
        pub env: Environment,
    }
);

json_struct!(
    /// Output returned by the `resolve_version` function.
    pub struct ResolveVersionOutput {
        /// New alias or version candidate to resolve.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub candidate: Option<String>,

        /// An explicitly resolved version to be used as-is.
        /// Note: Only use this field if you know what you're doing!
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<String>,
    }
);

// Shimmer

json_struct!(
    /// Configuration for individual shim files.
    pub struct ShimConfig {
        /// Relative path from the tool directory to the binary to execute.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bin_path: Option<PathBuf>,

        /// Name of a parent binary that's required for this shim to work.
        /// For example, `npm` requires `node`.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parent_bin: Option<String>,

        /// Custom args to prepend to user-provided args.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub before_args: Option<String>,

        /// Custom args to append to user-provided args.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub after_args: Option<String>,
    }
);

impl ShimConfig {
    /// Create a global shim that executes the parent tool,
    /// but uses the provided binary as the entry point.
    pub fn global_with_alt_bin<B>(bin_path: B) -> ShimConfig
    where
        B: AsRef<OsStr>,
    {
        ShimConfig {
            bin_path: Some(bin_path.as_ref().into()),
            ..ShimConfig::default()
        }
    }

    /// Create a global shim that executes the parent tool,
    /// but prefixes the user-provided arguments with the
    /// provided arguments (typically a sub-command).
    pub fn global_with_sub_command<A>(args: A) -> ShimConfig
    where
        A: AsRef<str>,
    {
        ShimConfig {
            before_args: Some(args.as_ref().to_owned()),
            ..ShimConfig::default()
        }
    }

    /// Create a local shim that executes the provided binary.
    pub fn local<B>(bin_path: B) -> ShimConfig
    where
        B: AsRef<OsStr>,
    {
        ShimConfig {
            bin_path: Some(bin_path.as_ref().into()),
            ..ShimConfig::default()
        }
    }

    /// Create a local shim that executes the provided binary
    /// through the context of the configured parent.
    pub fn local_with_parent<B, P>(bin_path: B, parent: P) -> ShimConfig
    where
        B: AsRef<OsStr>,
        P: AsRef<str>,
    {
        ShimConfig {
            bin_path: Some(bin_path.as_ref().into()),
            parent_bin: Some(parent.as_ref().to_owned()),
            ..ShimConfig::default()
        }
    }
}

json_struct!(
    /// Input passed to the `create_shims` function.
    pub struct CreateShimsInput {
        /// Current environment.
        pub env: Environment,
    }
);

json_struct!(
    /// Output returned by the `create_shims` function.
    pub struct CreateShimsOutput {
        /// Avoid creating the global shim.
        pub no_primary_global: bool,

        /// Configures the default/primary global shim.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub primary: Option<ShimConfig>,

        /// Additional global shims to create in the `~/.proto/bin` directory.
        /// Maps a shim name to a relative binary path.
        pub global_shims: HashMap<String, ShimConfig>,

        /// Local shims to create in the `~/.proto/tools/<id>/<version>/shims` directory.
        /// Maps a shim name to its configuration.
        pub local_shims: HashMap<String, ShimConfig>,
    }
);

json_struct!(
    /// Input passed to the `sync_manifest` function.
    pub struct SyncManifestInput {
        /// Current environment.
        pub env: Environment,

        /// Virtual path to the tool's installation directory.
        pub tool_dir: PathBuf,
    }
);

json_struct!(
    /// Output returned by the `sync_manifest` function.
    pub struct SyncManifestOutput {
        /// Override the default version with a new alias or version.
        pub default_version: Option<String>,

        /// List of versions that are currently installed. Will replace
        /// what is currently in the manifest.
        pub versions: Option<Vec<Version>>,
    }
);
