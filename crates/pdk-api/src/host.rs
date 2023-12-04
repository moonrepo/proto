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
