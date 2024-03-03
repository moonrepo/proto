use crate::api_struct;
use crate::virtual_path::VirtualPath;
use std::path::PathBuf;

pub use system_env::{SystemArch as HostArch, SystemLibc as HostLibc, SystemOS as HostOS};

api_struct!(
    /// Information about the host environment (the current runtime).
    pub struct HostEnvironment {
        pub arch: HostArch,
        pub libc: HostLibc,
        pub os: HostOS,
        pub home_dir: VirtualPath,
    }
);

api_struct!(
    /// Information about the current testing environment.
    pub struct TestEnvironment {
        pub ci: bool,
        pub sandbox: PathBuf,
    }
);
