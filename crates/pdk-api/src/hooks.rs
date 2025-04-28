use crate::api::ToolContext;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use warpgate_api::*;

api_struct!(
    /// Input passed to the `pre_install` and `post_install` hooks,
    /// while a `proto install` command is running.
    pub struct InstallHook {
        /// Current tool context.
        pub context: ToolContext,

        /// Whether the install was forced or not.
        pub forced: bool,

        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,

        /// Whether the resolved version was pinned.
        pub pinned: bool,

        /// Hide install output.
        pub quiet: bool,
    }
);

api_struct!(
    /// Input passed to the `pre_run` hook, before a `proto run` command
    /// or language binary is ran.
    pub struct RunHook {
        /// Current tool context.
        pub context: ToolContext,

        /// Path to the global packages directory for the tool, if found.
        pub globals_dir: Option<VirtualPath>,

        /// A prefix applied to the file names of globally installed packages.
        pub globals_prefix: Option<String>,

        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,
    }
);

api_struct!(
    /// Output returned from the `pre_run` hook.
    #[serde(default)]
    pub struct RunHookResult {
        /// Additional arguments to append to the running command.
        pub args: Option<Vec<String>>,

        /// Additional environment variables to pass to the running command.
        /// Will overwrite any existing variables.
        pub env: Option<FxHashMap<String, String>>,

        /// Additional paths to prepend to `PATH` for the running command.
        pub paths: Option<Vec<PathBuf>>,
    }
);
