mod build_source;

use crate::shapes::*;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use version_spec::{CalVer, SemVer, SpecError, UnresolvedVersionSpec, VersionSpec};
use warpgate_api::*;

pub use build_source::*;
pub use semver::{Version, VersionReq};

fn is_false(value: &bool) -> bool {
    !(*value)
}

api_struct!(
    /// Information about the current state of the tool.
    pub struct ToolContext {
        /// The version of proto (the core crate) calling plugin functions.
        #[serde(default)]
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
        #[serde(alias = "CLI")]
        CommandLine,
        #[default]
        Language,
        #[serde(alias = "PM")]
        DependencyManager,
        #[serde(alias = "VM")]
        VersionManager,
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

api_struct!(
    /// Output returned by the `register_tool` function.
    pub struct ToolMetadataOutput {
        /// Schema shape of the tool's configuration.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub config_schema: Option<schematic::Schema>,

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

// VERSION DETECTION

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
        pub context: ToolContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
    }
);

api_struct!(
    /// Output returned by the `native_install` function.
    pub struct NativeInstallOutput {
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
        pub context: ToolContext,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub archive_prefix: Option<String>,

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
    #[serde(default)]
    pub struct ExecutableConfig {
        /// The file to execute, relative from the tool directory.
        /// Does *not* support virtual paths.
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

        /// Whether this is the primary executable or not.
        #[serde(skip_serializing_if = "is_false")]
        pub primary: bool,

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

        /// Relative directory path from the tool install directory in which
        /// pre-installed executables can be located. This directory path
        /// will be used during `proto activate`, but not for bins/shims.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub exes_dir: Option<PathBuf>,

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
        #[deprecated(note = "Use `exes` instead.")]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub primary: Option<ExecutableConfig>,

        /// Configures secondary/additional executables to create.
        /// The map key is the name of the shim/binary file.
        #[deprecated(note = "Use `exes` instead.")]
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
        pub context: ToolContext,
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
