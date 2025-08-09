use crate::api::PluginContext;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use warpgate_api::*;

/// Enumeration of all available hook functions that can be implemented by plugins.
///
/// Hook functions are called at specific points during proto operations to allow
/// plugins to customize behavior, perform setup/cleanup, or modify the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookFunction {
    /// Pre-install hook.
    ///
    /// Called before a tool installation begins, allowing plugins to perform setup
    /// tasks, validate prerequisites, or modify the installation environment.
    ///
    /// **Input:** [`InstallHook`] | **Output:** None
    PreInstall,

    /// Post-install hook.
    ///
    /// Called after a tool installation completes successfully, allowing plugins
    /// to perform cleanup tasks, configure the tool, or set up additional resources.
    ///
    /// **Input:** [`InstallHook`] | **Output:** None
    PostInstall,

    /// Pre-run hook.
    ///
    /// Called before executing a tool binary, allowing plugins to modify environment
    /// variables, validate runtime conditions, or perform setup.
    ///
    /// **Input:** [`RunHook`] | **Output:** [`RunHookResult`]
    PreRun,
}

impl HookFunction {
    /// Get the string representation of the hook function name.
    ///
    /// This returns the actual function name that should be used when calling
    /// the hook function via WASM.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreInstall => "pre_install",
            Self::PostInstall => "post_install",
            Self::PreRun => "pre_run",
        }
    }
}

impl AsRef<str> for HookFunction {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

api_struct!(
    /// Input passed to the `pre_install` and `post_install` hooks,
    /// while a `proto install` command is running.
    pub struct InstallHook {
        /// Current tool context.
        pub context: PluginContext,

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
        pub context: PluginContext,

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
