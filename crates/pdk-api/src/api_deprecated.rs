#![allow(deprecated)]

use crate::api::ToolContext;
use crate::json_struct;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

json_struct!(
    /// Input passed to the `locate_bins` function.
    pub struct LocateBinsInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

json_struct!(
    /// Output returned by the `locate_bins` function.
    #[deprecated(since = "0.22.0", note = "Use `locate_executables` function instead.")]
    pub struct LocateBinsOutput {
        /// Relative path from the tool directory to the binary to execute.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bin_path: Option<PathBuf>,

        /// When true, the last item in `globals_lookup_dirs` will be used,
        /// regardless if it exists on the file system or not.
        pub fallback_last_globals_dir: bool,

        /// List of directory paths to find the globals installation directory.
        /// Each path supports environment variable expansion.
        pub globals_lookup_dirs: Vec<String>,

        /// A string that all global binaries are prefixed with, and will be removed
        /// when listing and filtering available globals.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub globals_prefix: Option<String>,
    }
);

// Shimmer

json_struct!(
    /// Configuration for individual shim files.
    pub struct ShimConfig {
        /// The binary to execute. Can be a relative path from the tool directory,
        /// or an absolute path
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bin_path: Option<PathBuf>,

        /// Name of a parent binary that's required for this shim to work.
        /// For example, `npm` requires `node`.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub parent_bin: Option<String>,

        /// Custom args to prepend to user-provided args.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub before_args: Option<String>,

        /// Custom args to append to user-provided args.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub after_args: Option<String>,
    }
);

impl ShimConfig {
    /// Create a global shim that executes the parent tool,
    /// but uses the provided binary as the entry point.
    pub fn global_with_alt_bin<B>(bin_path: B) -> ShimConfig
    where
        B: AsRef<OsStr>,
    {
        ShimConfig {
            bin_path: Some(bin_path.as_ref().into()),
            ..ShimConfig::default()
        }
    }

    /// Create a global shim that executes the parent tool,
    /// but prefixes the user-provided arguments with the
    /// provided arguments (typically a sub-command).
    pub fn global_with_sub_command<A>(args: A) -> ShimConfig
    where
        A: AsRef<str>,
    {
        ShimConfig {
            before_args: Some(args.as_ref().to_owned()),
            ..ShimConfig::default()
        }
    }

    /// Create a local shim that executes the provided binary.
    pub fn local<B>(bin_path: B) -> ShimConfig
    where
        B: AsRef<OsStr>,
    {
        ShimConfig {
            bin_path: Some(bin_path.as_ref().into()),
            ..ShimConfig::default()
        }
    }

    /// Create a local shim that executes the provided binary
    /// through the context of the configured parent.
    pub fn local_with_parent<B, P>(bin_path: B, parent: P) -> ShimConfig
    where
        B: AsRef<OsStr>,
        P: AsRef<str>,
    {
        ShimConfig {
            bin_path: Some(bin_path.as_ref().into()),
            parent_bin: Some(parent.as_ref().to_owned()),
            ..ShimConfig::default()
        }
    }
}

json_struct!(
    /// Input passed to the `create_shims` function.
    pub struct CreateShimsInput {
        /// Current tool context.
        pub context: ToolContext,
    }
);

json_struct!(
    /// Output returned by the `create_shims` function.
    #[deprecated(since = "0.22.0", note = "Use `locate_executables` function instead.")]
    pub struct CreateShimsOutput {
        /// Avoid creating the global shim.
        pub no_primary_global: bool,

        /// Configures the default/primary global shim.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub primary: Option<ShimConfig>,

        /// Additional global shims to create in the `~/.proto/shims` directory.
        /// Maps a shim name to a relative binary path.
        pub global_shims: HashMap<String, ShimConfig>,

        /// Local shims to create in the `~/.proto/tools/<id>/<version>/shims` directory.
        /// Maps a shim name to its configuration.
        pub local_shims: HashMap<String, ShimConfig>,
    }
);
