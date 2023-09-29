use crate::json_struct;
use serde::{Deserialize, Serialize};
use warpgate_api::VirtualPath;

pub use system_env::{SystemArch as HostArch, SystemOS as HostOS};

json_struct!(
    /// Information about the host environment (the current runtime).
    pub struct HostEnvironment {
        pub arch: HostArch,
        pub os: HostOS,
        pub home_dir: VirtualPath,
        pub proto_dir: VirtualPath,
    }
);

json_struct!(
    /// The current user's proto configuration.
    pub struct UserConfigSettings {
        pub auto_clean: bool,
        pub auto_install: bool,
        pub node_intercept_globals: bool,
    }
);
