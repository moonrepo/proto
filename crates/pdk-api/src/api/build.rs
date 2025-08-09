use super::source::*;
use crate::PluginContext;
use derive_setters::Setters;
use rustc_hash::FxHashMap;
use semver::VersionReq;
use std::path::PathBuf;
use system_env::SystemDependency;
use warpgate_api::{VirtualPath, api_enum, api_struct};

api_struct!(
    /// Input passed to the `build_instructions` function.
    pub struct BuildInstructionsInput {
        /// Current tool context.
        pub context: PluginContext,

        /// Virtual directory to install to.
        pub install_dir: VirtualPath,
    }
);

api_struct!(
    /// A builder and its parameters for installing the builder.
    #[derive(Setters)]
    pub struct BuilderInstruction {
        /// Unique identifier for this builder.
        #[setters(into)]
        pub id: String,

        /// Primary executable, relative from the source root.
        pub exe: PathBuf,

        /// Secondary executables, relative from the source root.
        #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
        pub exes: FxHashMap<String, PathBuf>,

        /// The Git source location for the builder.
        pub git: GitSource,
    }
);

api_struct!(
    /// A command and its parameters to be executed as a child process.
    #[derive(Setters)]
    pub struct CommandInstruction {
        /// The binary on `PATH`.
        #[setters(into)]
        pub bin: String,

        /// If the binary should reference a builder executable.
        pub builder: bool,

        /// List of arguments.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub args: Vec<String>,

        /// Map of environment variables.
        #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
        pub env: FxHashMap<String, String>,

        /// The working directory.
        #[setters(strip_option)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cwd: Option<PathBuf>,
    }
);

impl CommandInstruction {
    /// Create a new command with the binary and arguments.
    pub fn new<I: IntoIterator<Item = V>, V: AsRef<str>>(bin: &str, args: I) -> Self {
        Self {
            bin: bin.to_owned(),
            builder: false,
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
            env: FxHashMap::default(),
            cwd: None,
        }
    }

    /// Create a new command that executes a binary from a builder with the arguments.
    pub fn with_builder<I: IntoIterator<Item = V>, V: AsRef<str>>(id: &str, args: I) -> Self {
        let mut cmd = Self::new(id, args);
        cmd.builder = true;
        cmd
    }
}

api_enum!(
    /// An instruction to execute.
    #[serde(tag = "type", content = "instruction", rename_all = "kebab-case")]
    pub enum BuildInstruction {
        /// Install a builder locally that can be referenced in subsequent instructions.
        InstallBuilder(Box<BuilderInstruction>),

        /// Update a file and make it executable.
        MakeExecutable(PathBuf),

        /// Move a file from source to destination.
        MoveFile(PathBuf, PathBuf),

        /// Remove all files except those matching the provided list.
        RemoveAllExcept(Vec<PathBuf>),

        /// Remove a directory.
        RemoveDir(PathBuf),

        /// Remove a file.
        RemoveFile(PathBuf),

        /// Request (curl, wget, etc) a script and download to the host.
        RequestScript(String),

        /// Execute a command as a child process.
        #[cfg_attr(feature = "schematic", schema(nested))]
        RunCommand(Box<CommandInstruction>),

        /// Set an environment variable.
        SetEnvVar(String, String),
    }
);

api_enum!(
    /// Is required and must exist in the current environment.
    #[serde(tag = "type", content = "requirement", rename_all = "kebab-case")]
    pub enum BuildRequirement {
        CommandExistsOnPath(String),
        CommandVersion(String, VersionReq, Option<String>),
        ManualIntercept(String), // url
        GitConfigSetting(String, String),
        GitVersion(VersionReq),
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
        #[serde(skip_serializing_if = "Option::is_none")]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        pub source: Option<SourceLocation>,

        /// List of system dependencies that are required for building from source.
        /// If a dependency does not exist, it will be installed.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub system_dependencies: Vec<SystemDependency>,
    }
);
