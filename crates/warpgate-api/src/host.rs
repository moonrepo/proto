use crate::api_struct;
use crate::virtual_path::VirtualPath;

pub use system_env::{SystemArch as HostArch, SystemLibc as HostLibc, SystemOS as HostOS};

api_struct!(
    /// Information about the host environment (the current runtime).
    pub struct HostEnvironment {
        /// The architecture of the host system.
        pub arch: HostArch,

        /// Whether the plugin is running in a CI environment.
        pub ci: bool,

        /// The libc implementation of the host system.
        pub libc: HostLibc,

        /// The operating system of the host system.
        pub os: HostOS,

        /// Virtual path to the home directory of the current user.
        pub home_dir: VirtualPath,
    }
);

api_struct!(
    /// Information about the current testing environment.
    pub struct TestEnvironment {
        /// Whether the current test is running in a CI environment.
        pub ci: bool,

        /// Virtual path to the sandbox directory for the current test.
        pub sandbox: VirtualPath,
    }
);
