mod build;
mod checksum;
mod source;

use crate::shapes::*;
use derive_setters::Setters;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use version_spec::{CalVer, SemVer, SpecError, UnresolvedVersionSpec, VersionSpec};
use warpgate_api::*;

pub use build::*;
pub use checksum::*;
pub use semver::{Version, VersionReq};
pub use source::*;

/// Enumeration of all available plugin functions that can be implemented by plugins.
///
/// This enum provides type-safe access to plugin function names and eliminates
/// the risk of typos when calling plugin functions. Each variant corresponds to
/// a specific plugin function with its associated input/output types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginFunction {
    /// Register and configure a tool with proto.
    ///
    /// Called when proto first loads a plugin to get basic metadata about the tool
    /// including its name, type, and configuration schema.
    ///
    /// **Input:** [`RegisterToolInput`] | **Output:** [`RegisterToolOutput`]
    RegisterTool,

    /// Register a backend with proto.
    ///
    /// Allows plugins to define custom backends for sourcing tools from locations
    /// other than the default registry.
    ///
    /// **Input:** [`RegisterBackendInput`] | **Output:** [`RegisterBackendOutput`]
    RegisterBackend,

    /// Detect version files in a project.
    ///
    /// Returns a list of file patterns that should be checked for version information
    /// when auto-detecting tool versions.
    ///
    /// **Input:** [`DetectVersionInput`] | **Output:** [`DetectVersionOutput`]
    DetectVersionFiles,

    /// Parse version information from files.
    ///
    /// Extracts version specifications from configuration files like `.nvmrc`,
    /// `package.json`, `pyproject.toml`, etc.
    ///
    /// **Input:** [`ParseVersionFileInput`] | **Output:** [`ParseVersionFileOutput`]
    ParseVersionFile,

    /// Load available versions for a tool.
    ///
    /// Fetches the list of available versions that can be installed, including
    /// version aliases like "latest" or "lts".
    ///
    /// **Input:** [`LoadVersionsInput`] | **Output:** [`LoadVersionsOutput`]
    LoadVersions,

    /// Resolve version specifications to concrete versions.
    ///
    /// Takes version requirements or aliases and resolves them to specific
    /// installable versions.
    ///
    /// **Input:** [`ResolveVersionInput`] | **Output:** [`ResolveVersionOutput`]
    ResolveVersion,

    /// Download prebuilt tool archives.
    ///
    /// Provides URLs and metadata for downloading pre-compiled tool binaries
    /// instead of building from source.
    ///
    /// **Input:** [`DownloadPrebuiltInput`] | **Output:** [`DownloadPrebuiltOutput`]
    DownloadPrebuilt,

    /// Provide build instructions for tools.
    ///
    /// Returns the steps needed to build a tool from source, including dependencies,
    /// build commands, and environment requirements.
    ///
    /// **Input:** [`BuildInstructionsInput`] | **Output:** [`BuildInstructionsOutput`]
    BuildInstructions,

    /// Unpack downloaded archives.
    ///
    /// Handles custom unpacking logic for tool archives when the default extraction
    /// methods are insufficient.
    ///
    /// **Input:** [`UnpackArchiveInput`] | **Output:** None
    UnpackArchive,

    /// Verify download checksums.
    ///
    /// Provides custom checksum verification logic for downloaded tool archives
    /// to ensure integrity.
    ///
    /// **Input:** [`VerifyChecksumInput`] | **Output:** [`VerifyChecksumOutput`]
    VerifyChecksum,

    /// Native tool installation.
    ///
    /// Handles tool installation using the tool's own installation methods rather
    /// than proto's standard process.
    ///
    /// **Input:** [`NativeInstallInput`] | **Output:** [`NativeInstallOutput`]
    NativeInstall,

    /// Native tool uninstallation.
    ///
    /// Handles tool removal using the tool's own uninstallation methods rather
    /// than simple directory deletion.
    ///
    /// **Input:** [`NativeUninstallInput`] | **Output:** [`NativeUninstallOutput`]
    NativeUninstall,

    /// Locate tool executables.
    ///
    /// Identifies where executables are located within an installed tool and
    /// configures them for proto's shim system.
    ///
    /// **Input:** [`LocateExecutablesInput`] | **Output:** [`LocateExecutablesOutput`]
    LocateExecutables,

    /// Sync the tool manifest.
    ///
    /// Allows plugins to update proto's inventory of installed versions with
    /// external changes.
    ///
    /// **Input:** [`SyncManifestInput`] | **Output:** [`SyncManifestOutput`]
    SyncManifest,

    /// Sync shell profile configuration.
    ///
    /// Configures shell environment variables and PATH modifications needed for
    /// the tool to work properly.
    ///
    /// **Input:** [`SyncShellProfileInput`] | **Output:** [`SyncShellProfileOutput`]
    SyncShellProfile,
}

impl PluginFunction {
    /// Get the string representation of the plugin function name.
    ///
    /// This returns the actual function name that should be used when calling
    /// the plugin function via WASM.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RegisterTool => "register_tool",
            Self::RegisterBackend => "register_backend",
            Self::DetectVersionFiles => "detect_version_files",
            Self::ParseVersionFile => "parse_version_file",
            Self::LoadVersions => "load_versions",
            Self::ResolveVersion => "resolve_version",
            Self::DownloadPrebuilt => "download_prebuilt",
            Self::BuildInstructions => "build_instructions",
            Self::UnpackArchive => "unpack_archive",
            Self::VerifyChecksum => "verify_checksum",
            Self::NativeInstall => "native_install",
            Self::NativeUninstall => "native_uninstall",
            Self::LocateExecutables => "locate_executables",
            Self::SyncManifest => "sync_manifest",
            Self::SyncShellProfile => "sync_shell_profile",
        }
    }
}

impl AsRef<str> for PluginFunction {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

pub(crate) fn is_false(value: &bool) -> bool {
    !(*value)
}

api_struct!(
    /// Information about the current state of the plugin,
    /// after a version has been resolved.
    pub struct PluginContext {
        /// The version of proto (the core crate) calling plugin functions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub proto_version: Option<Version>,

        /// Virtual path to the tool's temporary directory.
        pub temp_dir: VirtualPath,

        /// Virtual path to the tool's installation directory.
        pub tool_dir: VirtualPath,

        /// Current version. Will be a "latest" alias if not resolved.
        pub version: VersionSpec,
    }
);

api_struct!(
    /// Information about the current state of the plugin,
    /// before a version has been resolved.
    pub struct PluginUnresolvedContext {
        /// The version of proto (the core crate) calling plugin functions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub proto_version: Option<Version>,

        /// Virtual path to the tool's temporary directory.
        pub temp_dir: VirtualPath,

        /// Current version if defined.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub version: Option<VersionSpec>,

        // TODO: temporary compat with `ToolContext`
        #[doc(hidden)]
        pub tool_dir: VirtualPath,
    }
);

api_unit_enum!(
    /// Supported types of plugins.
    pub enum PluginType {
        #[serde(alias = "CLI", alias = "CommandLine")] // TEMP
        CommandLine,
        #[default]
        #[serde(alias = "Language")]
        Language,
        #[serde(alias = "PM", alias = "DependencyManager")] // TEMP
        DependencyManager,
        #[serde(alias = "VM", alias = "VersionManager")] // TEMP
        VersionManager,
    }
);

api_struct!(
    /// Input passed to the `register_tool` function.
    pub struct RegisterToolInput {
        /// ID of the tool, as it was configured.
        pub id: String,
    }
);

#[deprecated(note = "Use `RegisterToolInput` instead.")]
pub type ToolMetadataInput = RegisterToolInput;

api_struct!(
    /// Controls aspects of the tool inventory.
    #[serde(default)]
    pub struct ToolInventoryMetadata {
        /// Override the tool inventory directory (where all versions are installed).
        /// This is an advanced feature and should only be used when absolutely necessary.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub override_dir: Option<VirtualPath>,

        /// Suffix to append to all versions when labeling directories.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub version_suffix: Option<String>,
    }
);

api_unit_enum!(
    /// Supported strategies for installing a tool.
    pub enum InstallStrategy {
        #[serde(alias = "BuildFromSource")] // TEMP
        BuildFromSource,
        #[default]
        #[serde(alias = "DownloadPrebuilt")] // TEMP
        DownloadPrebuilt,
    }
);

api_struct!(
    /// Output returned by the `register_tool` function.
    pub struct RegisterToolOutput {
        /// Schema shape of the tool's configuration.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub config_schema: Option<schematic::Schema>,

        /// Default strategy to use when installing a tool.
        #[serde(default)]
        pub default_install_strategy: InstallStrategy,

        /// Default alias or version to use as a fallback.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default_version: Option<UnresolvedVersionSpec>,

        /// List of deprecation messages that will be displayed to users
        /// of this plugin.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub deprecations: Vec<String>,

        /// Controls aspects of the tool inventory.
        #[serde(default)]
        pub inventory: ToolInventoryMetadata,

        /// Minimum version of proto required to execute this plugin.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub minimum_proto_version: Option<Version>,

        /// Human readable name of the tool.
        pub name: String,

        /// Version of the plugin.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub plugin_version: Option<Version>,

        /// Other plugins that this plugin requires.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub requires: Vec<String>,

        /// Names of commands that will self-upgrade the tool,
        /// and should be blocked from happening.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub self_upgrade_commands: Vec<String>,

        /// Type of the tool.
        #[serde(rename = "type")]
        pub type_of: PluginType,

        /// Whether this plugin is unstable or not.
        #[serde(default)]
        pub unstable: Switch,
    }
);

#[deprecated(note = "Use `RegisterToolOutput` instead.")]
pub type ToolMetadataOutput = RegisterToolOutput;

// BACKEND

api_struct!(
    /// Input passed to the `register_backend` function.
    pub struct RegisterBackendInput {
        /// Current tool context.
        pub context: PluginUnresolvedContext,

        /// ID of the tool, as it was configured.
        pub id: String,
    }
);

api_struct!(
    /// Output returned by the `register_backend` function.
    pub struct RegisterBackendOutput {
        /// Unique identifier for this backend. Will be used as the folder name.
        pub backend_id: String,

        /// List of executables, relative from the backend directory,
        /// that will be executed in the context of proto.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub exes: Vec<PathBuf>,

        /// Location in which to acquire source files for the backend.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub source: Option<SourceLocation>,
    }
);

// VERSION DETECTION

api_struct!(
    /// Input passed to the `detect_version_files` function.
    pub struct DetectVersionInput {
        /// Current tool context.
        pub context: PluginUnresolvedContext,
    }
);

api_struct!(
    /// Output returned by the `detect_version_files` function.
    #[serde(default)]
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

        /// Current tool context.
        pub context: PluginUnresolvedContext,

        /// Name of file that's being parsed.
        pub file: String,

        /// Virtual path to the file being parsed.
        pub path: VirtualPath,
    }
);

api_struct!(
    /// Output returned by the `parse_version_file` function.
    #[serde(default)]
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
        pub context: PluginContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
    }
);

api_struct!(
    /// Output returned by the `native_install` function.
    pub struct NativeInstallOutput {
        /// A checksum/hash that was generated.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub checksum: Option<Checksum>,

        /// Error message if the install failed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,

        /// Whether the install was successful.
        pub installed: bool,

        /// Whether to skip the install process or not.
        #[serde(default)]
        pub skip_install: bool,
    }
);

api_struct!(
    /// Input passed to the `native_uninstall` function.
    pub struct NativeUninstallInput {
        /// Current tool context.
        pub context: PluginContext,

        /// Virtual directory to uninstall from.
        pub uninstall_dir: VirtualPath,
    }
);

api_struct!(
    /// Output returned by the `native_uninstall` function.
    pub struct NativeUninstallOutput {
        /// Error message if the uninstall failed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,

        /// Whether the install was successful.
        pub uninstalled: bool,

        /// Whether to skip the uninstall process or not.
        #[serde(default)]
        pub skip_uninstall: bool,
    }
);

api_struct!(
    /// Input passed to the `download_prebuilt` function.
    pub struct DownloadPrebuiltInput {
        /// Current tool context.
        pub context: PluginContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
    }
);

api_struct!(
    /// Output returned by the `download_prebuilt` function.
    pub struct DownloadPrebuiltOutput {
        /// Name of the direct folder within the archive that contains the tool,
        /// and will be removed when unpacking the archive.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub archive_prefix: Option<String>,

        /// The checksum hash itself.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub checksum: Option<Checksum>,

        /// File name of the checksum to download. If not provided,
        /// will attempt to extract it from the URL.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub checksum_name: Option<String>,

        /// Public key to use for checksum verification.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub checksum_public_key: Option<String>,

        /// A secure URL to download the checksum file for verification.
        /// If the tool does not support checksum verification, this setting can be omitted.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub checksum_url: Option<String>,

        /// File name of the archive to download. If not provided,
        /// will attempt to extract it from the URL.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub download_name: Option<String>,

        /// A secure URL to download the tool/archive.
        pub download_url: String,
    }
);

api_struct!(
    /// Input passed to the `unpack_archive` function.
    pub struct UnpackArchiveInput {
        /// Current tool context.
        pub context: PluginContext,

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
        pub context: PluginContext,

        /// Virtual path to the checksum file.
        pub checksum_file: VirtualPath,

        /// A checksum of the downloaded file. The type of hash
        /// is derived from the checksum file's extension, otherwise
        /// it defaults to SHA256.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub download_checksum: Option<Checksum>,

        /// Virtual path to the downloaded file.
        pub download_file: VirtualPath,
    }
);

api_struct!(
    /// Output returned by the `verify_checksum` function.
    pub struct VerifyChecksumOutput {
        /// Was the checksum correct?
        pub verified: bool,
    }
);

// EXECUTABLES, BINARYS, GLOBALS

api_struct!(
    /// Input passed to the `locate_executables` function.
    pub struct LocateExecutablesInput {
        /// Current tool context.
        pub context: PluginContext,

        /// Virtual directory the tool was installed to.
        pub install_dir: VirtualPath,
    }
);

api_struct!(
    /// Configuration for generated shim and symlinked binary files.
    #[derive(Setters)]
    #[serde(default)]
    pub struct ExecutableConfig {
        /// The file to execute, relative from the tool directory.
        /// Does *not* support virtual paths.
        #[setters(strip_option)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub exe_path: Option<PathBuf>,

        /// The executable path to use for symlinking binaries instead of `exe_path`.
        /// This should only be used when `exe_path` is a non-standard executable.
        #[setters(strip_option)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub exe_link_path: Option<PathBuf>,

        /// Do not symlink a binary in `~/.proto/bin`.
        #[serde(skip_serializing_if = "is_false")]
        pub no_bin: bool,

        /// Do not generate a shim in `~/.proto/shims`.
        #[serde(skip_serializing_if = "is_false")]
        pub no_shim: bool,

        /// List of arguments to append to the parent executable, but prepend before
        /// all other arguments.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub parent_exe_args: Vec<String>,

        /// The parent executable name required to execute the local executable path.
        #[setters(into, strip_option)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parent_exe_name: Option<String>,

        /// Whether this is the primary executable or not.
        #[serde(skip_serializing_if = "is_false")]
        pub primary: bool,

        /// Custom args to prepend to user-provided args within the generated shim.
        #[setters(strip_option)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shim_before_args: Option<StringOrVec>,

        /// Custom args to append to user-provided args within the generated shim.
        #[setters(strip_option)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub shim_after_args: Option<StringOrVec>,

        /// Custom environment variables to set when executing the shim.
        #[setters(strip_option)]
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

    pub fn new_primary<T: AsRef<str>>(exe_path: T) -> Self {
        Self {
            exe_path: Some(PathBuf::from(exe_path.as_ref())),
            primary: true,
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
    #[serde(default)]
    pub struct LocateExecutablesOutput {
        /// Configures executable information to be used as proto bins/shims.
        /// The map key will be the name of the executable file.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub exes: FxHashMap<String, ExecutableConfig>,

        #[deprecated(note = "Use `exes_dirs` instead.")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub exes_dir: Option<PathBuf>,

        /// Relative directory path from the tool install directory in which
        /// pre-installed executables can be located. This directory path
        /// will be used during `proto activate`, but not for bins/shims.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub exes_dirs: Vec<PathBuf>,

        /// List of directory paths to find the globals installation directory.
        /// Each path supports environment variable expansion.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub globals_lookup_dirs: Vec<String>,

        /// A string that all global binaries are prefixed with, and will be removed
        /// when listing and filtering available globals.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub globals_prefix: Option<String>,
    }
);

// VERSION RESOLVING

api_struct!(
    /// Input passed to the `load_versions` function.
    pub struct LoadVersionsInput {
        /// Current tool context.
        pub context: PluginUnresolvedContext,

        /// The alias or version currently being resolved.
        pub initial: UnresolvedVersionSpec,
    }
);

api_struct!(
    /// Output returned by the `load_versions` function.
    #[serde(default)]
    pub struct LoadVersionsOutput {
        /// Latest canary version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub canary: Option<UnresolvedVersionSpec>,

        /// Latest stable version.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub latest: Option<UnresolvedVersionSpec>,

        /// Mapping of aliases (channels, etc) to a version.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub aliases: FxHashMap<String, UnresolvedVersionSpec>,

        /// List of available production versions to install.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub versions: Vec<VersionSpec>,
    }
);

impl LoadVersionsOutput {
    /// Create the output from a list of strings that'll be parsed as versions.
    /// The latest version will be the highest version number.
    pub fn from(values: Vec<String>) -> Result<Self, SpecError> {
        let mut versions = vec![];

        for value in values {
            versions.push(VersionSpec::parse(&value)?);
        }

        Ok(Self::from_versions(versions))
    }

    /// Create the output from a list of version specifications.
    /// The latest version will be the highest version number.
    pub fn from_versions(versions: Vec<VersionSpec>) -> Self {
        let mut output = LoadVersionsOutput::default();
        let mut latest = Version::new(0, 0, 0);
        let mut calver = false;

        for version in versions {
            if let Some(inner) = version.as_version() {
                if inner.pre.is_empty() && inner.build.is_empty() && inner > &latest {
                    inner.clone_into(&mut latest);
                    calver = matches!(version, VersionSpec::Calendar(_));
                }
            }

            output.versions.push(version);
        }

        output.latest = Some(if calver {
            UnresolvedVersionSpec::Calendar(CalVer(latest))
        } else {
            UnresolvedVersionSpec::Semantic(SemVer(latest))
        });

        output
            .aliases
            .insert("latest".into(), output.latest.clone().unwrap());

        output
    }
}

api_struct!(
    /// Input passed to the `resolve_version` function.
    pub struct ResolveVersionInput {
        /// Current tool context.
        pub context: PluginUnresolvedContext,

        /// The alias or version currently being resolved.
        pub initial: UnresolvedVersionSpec,
    }
);

api_struct!(
    /// Output returned by the `resolve_version` function.
    #[serde(default)]
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
        pub context: PluginContext,
    }
);

api_struct!(
    /// Output returned by the `sync_manifest` function.
    #[serde(default)]
    pub struct SyncManifestOutput {
        /// List of versions that are currently installed. Will replace
        /// what is currently in the manifest.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub versions: Option<Vec<VersionSpec>>,

        /// Whether to skip the syncing process or not.
        pub skip_sync: bool,
    }
);

api_struct!(
    /// Input passed to the `sync_shell_profile` function.
    pub struct SyncShellProfileInput {
        /// Current tool context.
        pub context: PluginContext,

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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub export_vars: Option<FxHashMap<String, String>>,

        /// A list of paths to prepend to the `PATH` environment variable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub extend_path: Option<Vec<String>>,

        /// Whether to skip the syncing process or not.
        #[serde(default)]
        pub skip_sync: bool,
    }
);
