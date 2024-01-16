use warpgate_api::*;

pub use system_env::{SystemArch as HostArch, SystemOS as HostOS};

api_struct!(
    /// Information about the host environment (the current runtime).
    pub struct HostEnvironment {
        pub arch: HostArch,
        pub os: HostOS,
        pub home_dir: VirtualPath,
    }
);
