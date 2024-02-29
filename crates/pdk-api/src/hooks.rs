use crate::api::ToolContext;
use rustc_hash::FxHashMap;
use warpgate_api::*;

api_struct!(
    /// Input passed to the `pre_install` and `post_install` hooks,
    /// while a `proto install` command is running.
    pub struct InstallHook {
        /// Current tool context.
        pub context: ToolContext,

        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,

        /// Whether the resolved version was pinned
        pub pinned: bool,
    }
);

api_struct!(
    /// Input passed to the `pre_run` hook, before a `proto run` command
    /// or language binary is ran.
    pub struct RunHook {
        /// Current tool context.
        pub context: ToolContext,

        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,
    }
);

api_struct!(
    /// Output returned from the `pre_run` hook.
    pub struct RunHookResult {
        /// Additional arguments to append to the running command.
        pub args: Option<Vec<String>>,

        /// Additional environment variables to pass to the running command.
        /// Will overwrite any existing variables.
        pub env: Option<FxHashMap<String, String>>,
    }
);
