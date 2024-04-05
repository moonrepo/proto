use crate::shapes::StringOrVec;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use system_env::SystemDependency;
use version_spec::{UnresolvedVersionSpec, VersionSpec};
use warpgate_api::*;

pub use semver::{Version, VersionReq};

fn is_false(value: &bool) -> bool {
    !(*value)
}

api_struct!(
    /// Information about the current state of the tool.
    pub struct ToolContext {
        /// The version of proto (the core crate) calling plugin functions.
        pub proto_version: Option<Version>,

        /// Virtual path to the tool's installation directory.
        pub tool_dir: VirtualPath,

        /// Current version. Will be a "latest" alias if not resolved.
        pub version: VersionSpec,
    }
);

api_enum!(
    /// Supported types of plugins.
    #[derive(Default)]
    pub enum PluginType {
        #[default]
        Language,
        DependencyManager,
        CLI,
    }
);

api_struct!(
    /// Input passed to the `register_tool` function.
    pub struct ToolMetadataInput {
        /// ID of the tool, as it was configured.
        pub id: String,
    }
);

api_struct!(
    /// Controls aspects of the tool inventory.
    pub struct ToolInventoryMetadata {
        /// Disable progress bars when installing or uninstalling tools.
        #[serde(skip_serializing_if = "is_false")]
        pub disable_progress_bars: bool,

        /// Override the tool inventory directory (where all versions are installed).
        /// This is an advanced feature and should only be used when absolutely necessary.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub override_dir: Option<VirtualPath>,

        /// Suffix to append to all versions when labeling directories.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version_suffix: Option<String>,
    }
);

api_struct!(
    /// Output returned by the `register_tool` function.
    pub struct ToolMetadataOutput {
        /// Default alias or version to use as a fallback.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_version: Option<UnresolvedVersionSpec>,

        /// Controls aspects of the tool inventory.
        pub inventory: ToolInventoryMetadata,

        /// Human readable name of the tool.
        pub name: String,

        /// Version of the plugin.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub plugin_version: Option<String>,

        /// Names of commands that will self-upgrade the tool,
        /// and should be blocked from happening.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub self_upgrade_commands: Vec<String>,

        /// Type of the tool.
        #[serde(rename = "type")]
        pub type_of: PluginType,
    }
);

// VERSION DETECTION

api_struct!(
    /// Output returned by the `detect_version_files` function.
    pub struct DetectVersionOutput {
        /// List of files that should be checked for version information.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub files: Vec<String>,

        /// List of path patterns to ignore when traversing directories.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub ignore: Vec<String>,
    }
);

api_struct!(
    /// Input passed to the `parse_version_file` function.
    pub struct ParseVersionFileInput {
        /// File contents to parse/extract a version from.
        pub content: String,

        /// Name of file that's being parsed.
        pub file: String,
    }
);

api_struct!(
    /// Output returned by the `parse_version_file` function.
    pub struct ParseVersionFileOutput {
        /// The version that was extracted from the file.
        /// Can be a semantic version or a version requirement/range.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<UnresolvedVersionSpec>,
    }
);

// DOWNLOAD, BUILD, INSTALL, VERIFY

api_struct!(
    /// Input passed to the `native_install` function.
    pub struct NativeInstallInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
    }
);

api_struct!(
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

api_struct!(
    /// Input passed to the `native_uninstall` function.
    pub struct NativeUninstallInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

api_struct!(
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

api_struct!(
    /// Input passed to the `build_instructions` function.
    pub struct BuildInstructionsInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

api_enum!(
    #[derive(Default)]
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum SourceLocation {
        #[default]
        None,
        // Archive {
        //     url: String,
        // },
        // Git {
        //     url: String,
        //     reference: String,
        //     submodules: bool,
        // },
    }
);

api_enum!(
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum BuildInstruction {
        None,
        // Command {
        //     bin: String,
        //     args: Vec<String>,
        //     env: FxHashMap<String, String>,
        // },
    }
);

api_struct!(
    /// Output returned by the `build_instructions` function.
    pub struct BuildInstructionsOutput {
        /// Link to the documentation/help.
        pub help_url: Option<String>,

        /// Location in which to acquire the source files. Can be an archive URL,
        /// or Git repository.
        pub source: SourceLocation,

        /// List of instructions to execute to build the tool, after system
        /// dependencies have been installed.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub instructions: Vec<BuildInstruction>,

        /// List of system dependencies that are required for building from source.
        /// If a dependency does not exist, it will be installed.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub system_dependencies: Vec<SystemDependency>,
    }
);

api_struct!(
    /// Input passed to the `download_prebuilt` function.
    pub struct DownloadPrebuiltInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
    }
);

api_struct!(
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

api_struct!(
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

api_struct!(
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

api_struct!(
    /// Output returned by the `verify_checksum` function.
    pub struct VerifyChecksumOutput {
        pub verified: bool,
    }
);

// EXECUTABLES, BINARYS, GLOBALS

api_struct!(
    /// Input passed to the `locate_executables` function.
    pub struct LocateExecutablesInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

api_struct!(
    /// Configuration for generated shim and symlinked binary files.
    pub struct ExecutableConfig {
        /// The file to execute, relative from the tool directory.
        /// Does *not* support virtual paths.
        ///
        /// The following scenarios are powered by this field:
        /// - Is the primary executable.
        /// - For primary and secondary bins, the source file to be symlinked,
        ///   and the extension to use for the symlink file itself.
        /// - For primary shim, this field is ignored.
        /// - For secondary shims, the file to execute.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub exe_path: Option<PathBuf>,

        /// The executable path to use for symlinking binaries instead of `exe_path`.
        /// This should only be used when `exe_path` is a non-standard executable.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub exe_link_path: Option<PathBuf>,

        /// Do not symlink a binary in `~/.proto/bin`.
        #[serde(skip_serializing_if = "is_false")]
        pub no_bin: bool,

        /// Do not generate a shim in `~/.proto/shims`.
        #[serde(skip_serializing_if = "is_false")]
        pub no_shim: bool,

        /// The parent executable name required to execute the local executable path.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parent_exe_name: Option<String>,

        /// Custom args to prepend to user-provided args within the generated shim.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shim_before_args: Option<StringOrVec>,

        /// Custom args to append to user-provided args within the generated shim.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shim_after_args: Option<StringOrVec>,

        /// Custom environment variables to set when executing the shim.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shim_env_vars: Option<FxHashMap<String, String>>,
    }
);

impl ExecutableConfig {
    pub fn new<T: AsRef<str>>(exe_path: T) -> Self {
        Self {
            exe_path: Some(PathBuf::from(exe_path.as_ref())),
            ..ExecutableConfig::default()
        }
    }

    pub fn with_parent<T: AsRef<str>, P: AsRef<str>>(exe_path: T, parent_exe: P) -> Self {
        Self {
            exe_path: Some(PathBuf::from(exe_path.as_ref())),
            parent_exe_name: Some(parent_exe.as_ref().to_owned()),
            ..ExecutableConfig::default()
        }
    }
}

api_struct!(
    /// Output returned by the `locate_executables` function.
    pub struct LocateExecutablesOutput {
        /// List of directory paths to find the globals installation directory.
        /// Each path supports environment variable expansion.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub globals_lookup_dirs: Vec<String>,

        /// A string that all global binaries are prefixed with, and will be removed
        /// when listing and filtering available globals.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub globals_prefix: Option<String>,

        /// Configures the primary/default executable to create.
        /// If not provided, a primary shim and binary will *not* be created.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub primary: Option<ExecutableConfig>,

        /// Configures secondary/additional executables to create.
        /// The map key is the name of the shim/binary file.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub secondary: FxHashMap<String, ExecutableConfig>,
    }
);

// VERSION RESOLVING

api_struct!(
    /// Input passed to the `load_versions` function.
    pub struct LoadVersionsInput {
        /// The alias or version currently being resolved.
        pub initial: UnresolvedVersionSpec,
    }
);

api_struct!(
    /// Output returned by the `load_versions` function.
    pub struct LoadVersionsOutput {
        /// Latest canary version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub canary: Option<Version>,

        /// Latest stable version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latest: Option<Version>,

        /// Mapping of aliases (channels, etc) to a version.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub aliases: FxHashMap<String, Version>,

        /// List of available production versions to install.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub versions: Vec<Version>,
    }
);

impl LoadVersionsOutput {
    /// Create the output from a list of strings that'll be parsed as versions.
    /// The latest version will be the highest version number.
    pub fn from(values: Vec<String>) -> Result<Self, semver::Error> {
        let mut versions = vec![];

        for value in values {
            versions.push(Version::parse(&value)?);
        }

        Ok(Self::from_versions(versions))
    }

    /// Create the output from a list of versions.
    /// The latest version will be the highest version number.
    pub fn from_versions(versions: Vec<Version>) -> Self {
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
        output
    }
}

api_struct!(
    /// Input passed to the `resolve_version` function.
    pub struct ResolveVersionInput {
        /// The alias or version currently being resolved.
        pub initial: UnresolvedVersionSpec,
    }
);

api_struct!(
    /// Output returned by the `resolve_version` function.
    pub struct ResolveVersionOutput {
        /// New alias or version candidate to resolve.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub candidate: Option<UnresolvedVersionSpec>,

        /// An explicitly resolved version to be used as-is.
        /// Note: Only use this field if you know what you're doing!
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version: Option<VersionSpec>,
    }
);

// MISCELLANEOUS

api_struct!(
    /// Input passed to the `sync_manifest` function.
    pub struct SyncManifestInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

api_struct!(
    /// Output returned by the `sync_manifest` function.
    pub struct SyncManifestOutput {
        /// List of versions that are currently installed. Will replace
        /// what is currently in the manifest.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub versions: Option<Vec<Version>>,

        /// Whether to skip the syncing process or not.
        pub skip_sync: bool,
    }
);

api_struct!(
    /// Input passed to the `sync_shell_profile` function.
    pub struct SyncShellProfileInput {
        /// Current tool context.
        pub context: ToolContext,

        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,
    }
);

api_struct!(
    /// Output returned by the `sync_shell_profile` function.
    pub struct SyncShellProfileOutput {
        /// An environment variable to check for in the shell profile.
        /// If the variable exists, injecting path and exports will be avoided.
        pub check_var: String,

        /// A mapping of environment variables that will be injected as exports.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub export_vars: Option<FxHashMap<String, String>>,

        /// A list of paths to prepend to the `PATH` environment variable.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub extend_path: Option<Vec<String>>,

        /// Whether to skip the syncing process or not.
        pub skip_sync: bool,
    }
);
