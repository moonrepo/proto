use crate::{json_struct, ToolContext};
use serde::{Deserialize, Serialize};

json_struct!(
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

json_struct!(
    /// Input passed to the `pre_run` hook, before a `proto run` command
    /// or language binary is ran.
    pub struct RunHook {
        /// Current tool context.
        pub context: ToolContext,

        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,
    }
);
