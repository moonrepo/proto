use crate::json_struct;
use serde::{Deserialize, Serialize};

json_struct!(
    /// Input passed to the `pre_install` and `post_install` hooks,
    /// while a `proto install` command is running.
    pub struct InstallHook {
        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,

        /// Whether the resolved version was pinned
        pub pinned: bool,

        /// Resolved and semantic version that's being installed.
        pub resolved_version: String,
    }
);

json_struct!(
    /// Input passed to the `pre_run` and `post_run` hooks,
    /// while a `proto run` command or language binary is running.
    pub struct RunHook {
        /// Arguments passed after `--` that was directly passed to the tool's binary.
        pub passthrough_args: Vec<String>,

        /// Resolved and semantic version of tool running.
        pub resolved_version: String,
    }
);
