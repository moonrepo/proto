use crate::host_funcs::ExecCommandOutput;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use system_env::SystemDependency;
use warpgate_api::VirtualPath;

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
    /// Information about the current state of the tool.
    pub struct ToolContext {
        /// Requested environment variables. Only non-empty values are included.
        pub env_vars: HashMap<String, String>,

        /// Virtual path to the tool's installation directory.
        pub tool_dir: VirtualPath,

        /// Current version. Will be empty if not resolved.
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

        /// Version of the plugin.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub plugin_version: Option<String>,

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
        /// Current tool context.
        pub context: ToolContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
    }
);

json_struct!(
    /// Output returned by the `native_install` function.
    pub struct NativeInstallOutput {
        /// Error message if the install failed.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,

        /// Whether the install was successful.
        pub installed: bool,

        /// Whether to skip the install process or not.
        pub skip_install: bool,
    }
);

json_struct!(
    /// Input passed to the `native_uninstall` function.
    pub struct NativeUninstallInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

json_struct!(
    /// Output returned by the `native_uninstall` function.
    pub struct NativeUninstallOutput {
        /// Error message if the uninstall failed.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,

        /// Whether the install was successful.
        pub uninstalled: bool,

        /// Whether to skip the uninstall process or not.
        pub skip_uninstall: bool,
    }
);

json_struct!(
    /// Input passed to the `build_instructions` function.
    pub struct BuildInstructionsInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

json_enum!(
    #[derive(Default)]
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum SourceLocation {
        #[default]
        None,
        Archive {
            url: String,
        },
        Git {
            url: String,
            reference: String,
            submodules: bool,
        },
    }
);

json_enum!(
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum BuildInstruction {
        Command {
            bin: String,
            args: Vec<String>,
            env: HashMap<String, String>,
        },
    }
);

json_struct!(
    /// Output returned by the `build_instructions` function.
    pub struct BuildInstructionsOutput {
        /// Link to the documentation/help.
        pub help_url: Option<String>,

        /// Location in which to acquire the source files. Can be an archive URL,
        /// or Git repository.
        pub source: SourceLocation,

        /// List of instructions to execute to build the tool, after system
        /// dependencies have been installed.
        pub instructions: Vec<BuildInstruction>,

        /// List of system dependencies that are required for building from source.
        /// If a dependency does not exist, it will be installed.
        pub system_dependencies: Vec<SystemDependency>,
    }
);

json_struct!(
    /// Input passed to the `download_prebuilt` function.
    pub struct DownloadPrebuiltInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
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

        /// Public key to use for checksum verification.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub checksum_public_key: Option<String>,

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
        /// Current tool context.
        pub context: ToolContext,

        /// Virtual path to the downloaded file.
        pub input_file: VirtualPath,

        /// Virtual directory to unpack the archive into, or copy the binary to.
        pub output_dir: VirtualPath,
    }
);

json_struct!(
    /// Output returned by the `verify_checksum` function.
    pub struct VerifyChecksumInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Virtual path to the checksum file.
        pub checksum_file: VirtualPath,

        /// Virtual path to the downloaded file.
        pub download_file: VirtualPath,
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
        /// Current tool context.
        pub context: ToolContext,
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

json_struct!(
    /// Input passed to the `install_global` function.
    pub struct InstallGlobalInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Name (and optional version) of the global dependency to install.
        pub dependency: String,

        /// Virtual path to the global's installation directory.
        pub globals_dir: VirtualPath,
    }
);

json_struct!(
    /// Output returned by the `install_global` function.
    pub struct InstallGlobalOutput {
        /// Error message if the install failed.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,

        /// Whether the install was successful.
        pub installed: bool,
    }
);

impl InstallGlobalOutput {
    pub fn from_exec_command(result: ExecCommandOutput) -> Self {
        if result.exit_code == 0 {
            return Self {
                installed: true,
                error: None,
            };
        }

        Self {
            installed: false,
            error: Some(result.stderr),
        }
    }
}

json_struct!(
    /// Input passed to the `uninstall_global` function.
    pub struct UninstallGlobalInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Name (and optional version) of the global dependency to uninstall.
        pub dependency: String,

        /// Virtual path to the global's installation directory.
        pub globals_dir: VirtualPath,
    }
);

json_struct!(
    /// Output returned by the `uninstall_global` function.
    pub struct UninstallGlobalOutput {
        /// Whether the uninstall was successful.
        pub uninstalled: bool,

        /// Error message if the uninstall failed.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
    }
);

impl UninstallGlobalOutput {
    pub fn from_exec_command(result: ExecCommandOutput) -> Self {
        if result.exit_code == 0 {
            return Self {
                uninstalled: true,
                error: None,
            };
        }

        Self {
            uninstalled: false,
            error: Some(result.stderr),
        }
    }
}

// Resolver

json_struct!(
    /// Input passed to the `load_versions` function.
    pub struct LoadVersionsInput {
        /// Current tool context.
        pub context: ToolContext,

        /// The alias or version currently being resolved.
        pub initial: String,
    }
);

json_struct!(
    /// Output returned by the `load_versions` function.
    pub struct LoadVersionsOutput {
        /// Latest canary version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub canary: Option<Version>,

        /// Latest stable version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latest: Option<Version>,

        /// Mapping of aliases (channels, etc) to a version.
        pub aliases: HashMap<String, Version>,

        /// List of available production versions to install.
        pub versions: Vec<Version>,
    }
);

impl LoadVersionsOutput {
    #[deprecated = "Use from() instead."]
    pub fn from_tags(tags: &[String]) -> anyhow::Result<Self> {
        Self::from(tags.to_vec())
    }

    /// Create the output from a list of strings that'll be parsed as versions.
    /// The latest version will be the highest version number.
    pub fn from(values: Vec<String>) -> anyhow::Result<Self> {
        let mut versions = vec![];

        for value in values {
            versions.push(Version::parse(&value)?);
        }

        Self::from_versions(versions)
    }

    /// Create the output from a list of versions.
    /// The latest version will be the highest version number.
    pub fn from_versions(versions: Vec<Version>) -> anyhow::Result<Self> {
        let mut output = LoadVersionsOutput::default();
        let mut latest = Version::new(0, 0, 0);

        for version in versions {
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
        /// Current tool context.
        pub context: ToolContext,

        /// The alias or version currently being resolved.
        pub initial: String,
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
        /// Current tool context.
        pub context: ToolContext,
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

        /// Additional global shims to create in the `~/.proto/shims` directory.
        /// Maps a shim name to a relative binary path.
        pub global_shims: HashMap<String, ShimConfig>,

        /// Local shims to create in the `~/.proto/tools/<id>/<version>/shims` directory.
        /// Maps a shim name to its configuration.
        pub local_shims: HashMap<String, ShimConfig>,
    }
);

// Misc

json_struct!(
    /// Input passed to the `sync_manifest` function.
    pub struct SyncManifestInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

json_struct!(
    /// Output returned by the `sync_manifest` function.
    pub struct SyncManifestOutput {
        /// Override the default version with a new alias or version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_version: Option<String>,

        /// List of versions that are currently installed. Will replace
        /// what is currently in the manifest.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub versions: Option<Vec<Version>>,

        /// Whether to skip the syncing process or not.
        pub skip_sync: bool,
    }
);

json_struct!(
    /// Input passed to the `sync_shell_profile` function.
    pub struct SyncShellProfileInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,
    }
);

json_struct!(
    /// Output returned by the `sync_shell_profile` function.
    pub struct SyncShellProfileOutput {
        /// An environment variable to check for in the shell profile.
        /// If the variable exists, injecting path and exports will be avoided.
        pub check_var: String,

        /// A mapping of environment variables that will be injected as exports.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub export_vars: Option<HashMap<String, String>>,

        /// A list of paths to prepend to the `PATH` environment variable.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub extend_path: Option<Vec<String>>,

        /// Whether to skip the syncing process or not.
        pub skip_sync: bool,
    }
);
