use crate::ToolContext;
use rustc_hash::FxHashMap;
use semver::VersionReq;
use std::path::PathBuf;
use system_env::SystemDependency;
use warpgate_api::{api_enum, api_struct};

api_struct!(
    /// Input passed to the `build_instructions` function.
    pub struct BuildInstructionsInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

api_struct!(
    /// Source code is contained in an archive.
    pub struct ArchiveSource {
        /// The URL to download the archive from.
        pub url: String,

        /// A path prefix within the archive to remove.
        #[serde(default)]
        pub prefix: Option<String>,
    }
);

api_struct!(
    /// Source code is located in a Git repository.
    pub struct GitSource {
        /// The URL of the Git remote.
        pub url: String,

        /// The branch/commit/tag to checkout.
        pub reference: String,

        /// Include submodules during checkout.
        #[serde(default)]
        pub submodules: bool,
    }
);

api_enum!(
    /// The location in which source code can be acquired.
    #[serde(tag = "type", rename_all = "kebab-case")]
    pub enum SourceLocation {
        /// Downloaded from an archive.
        #[cfg_attr(feature = "schematic", schema(nested))]
        Archive(ArchiveSource),

        /// Cloned from a Git repository.
        #[cfg_attr(feature = "schematic", schema(nested))]
        Git(GitSource),
    }
);

api_struct!(
    /// A command and its parameters to be executed as a child process.
    pub struct CommandInstruction {
        /// The binary on `PATH`.
        pub bin: String,

        /// List of arguments.
        pub args: Vec<String>,

        /// Map of environment variables.
        #[serde(default)]
        pub env: FxHashMap<String, String>,

        /// The working directory.
        #[serde(default)]
        pub cwd: Option<PathBuf>,
    }
);

impl CommandInstruction {
    /// Create a new command.
    pub fn new<I: IntoIterator<Item = V>, V: AsRef<str>>(bin: &str, args: I) -> Self {
        Self {
            bin: bin.to_owned(),
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
            env: FxHashMap::default(),
            cwd: None,
        }
    }
}

api_enum!(
    /// An instruction to execute.
    #[serde(tag = "type", content = "instruction", rename_all = "kebab-case")]
    pub enum BuildInstruction {
        /// Update a file and make it executable.
        MakeExecutable(PathBuf),

        /// Move a file from source to destination.
        MoveFile(PathBuf, PathBuf),

        /// Remove a directory.
        RemoveDir(PathBuf),

        /// Remove a file.
        RemoveFile(PathBuf),

        /// Request (curl, wget, etc) a script and download to the host.
        RequestScript(String),

        /// Execute a command as a child process.
        #[cfg_attr(feature = "schematic", schema(nested))]
        RunCommand(Box<CommandInstruction>),
    }
);

api_enum!(
    /// Is required and must exist in the current environment.
    #[serde(tag = "type", content = "requirement", rename_all = "kebab-case")]
    pub enum BuildRequirement {
        CommandExistsOnPath(String),
        ManualIntercept(String), // url
        GitConfigSetting(String, String),
        GitVersion(VersionReq),
        PythonVersion(VersionReq),
        RubyVersion(VersionReq),
        // macOS
        XcodeCommandLineTools,
        // Windows
        WindowsDeveloperMode,
    }
);

api_struct!(
    /// Output returned by the `build_instructions` function.
    #[serde(default)]
    pub struct BuildInstructionsOutput {
        /// Link to the documentation/help.
        pub help_url: Option<String>,

        /// List of instructions to execute to build the tool, after system
        /// dependencies have been installed.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub instructions: Vec<BuildInstruction>,

        /// List of requirements that must be met before dependencies are
        /// installed and instructions are executed.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub requirements: Vec<BuildRequirement>,

        /// Location in which to acquire the source files.
        pub source: Option<SourceLocation>,

        /// List of system dependencies that are required for building from source.
        /// If a dependency does not exist, it will be installed.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub system_dependencies: Vec<SystemDependency>,
    }
);
