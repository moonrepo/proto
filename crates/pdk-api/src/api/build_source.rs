use crate::ToolContext;
use rustc_hash::FxHashMap;
use semver::VersionReq;
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
    /// Source code is located in a Git repository.
    pub struct GitSource {
        pub url: String,
        pub reference: String,
        pub submodules: bool,
    }
);

api_enum!(
    #[derive(Default)]
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum SourceLocation {
        #[default]
        None,

        #[cfg_attr(feature = "schematic", schema(nested))]
        Git(GitSource),
    }
);

api_struct!(
    pub struct CommandInstruction {
        pub bin: String,
        pub args: Vec<String>,
        pub env: FxHashMap<String, String>,
    }
);

impl CommandInstruction {
    pub fn new<I: IntoIterator<Item = V>, V: AsRef<str>>(bin: &str, args: I) -> Self {
        Self {
            bin: bin.to_owned(),
            args: args
                .into_iter()
                .map(|arg| arg.as_ref().to_owned())
                .collect(),
            env: FxHashMap::default(),
        }
    }
}

api_enum!(
    #[serde(tag = "type", rename_all = "lowercase")]
    pub enum BuildInstruction {
        None,

        #[cfg_attr(feature = "schematic", schema(nested))]
        RunCommand(CommandInstruction),
    }
);

api_enum!(
    #[serde(rename_all = "lowercase")]
    pub enum BuildRequirement {
        CommandExistsOnPath(String),
        ManualIntercept(String), // url
        GitConfigSetting(String, String),
        GitVersion(VersionReq),
        PythonVersion(VersionReq),
        // macOS
        XcodeCommandLineTools,
        // Windows
        WindowsDeveloperMode,
    }
);

api_struct!(
    /// Output returned by the `build_instructions` function.
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
        pub source: SourceLocation,

        /// List of system dependencies that are required for building from source.
        /// If a dependency does not exist, it will be installed.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub system_dependencies: Vec<SystemDependency>,
    }
);
