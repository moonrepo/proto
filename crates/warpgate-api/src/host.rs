use crate::api_struct;
use crate::virtual_path::VirtualPath;

pub use system_env::{
    SystemArch as HostArch, SystemDetails as HostDetails, SystemDistro as HostDistro,
    SystemLibc as HostLibc, SystemOS as HostOS, SystemPlatform as HostPlatform,
};

api_struct!(
    /// Information about the host environment (the current runtime).
    pub struct HostEnvironment {
        /// The architecture of the host system.
        pub arch: HostArch,

        /// Whether the plugin is running in a CI environment.
        pub ci: bool,

        /// Details about the hardware and operating system of the host system,
        /// like the x86-64 microarchitecture level or Linux distribution, that
        /// plugins can use to choose between builds for the same platform.
        ///
        /// Not available when targeting another platform (when `foreign` is
        /// true), or when the host doesn't provide them (for example, older
        /// versions), so plugins must fall back to a default.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub details: Option<HostDetails>,

        /// Whether the plugin is targeting a platform other than the machine
        /// it's running on, for example, when locking other platforms. When
        /// true, the architecture, operating system, and libc describe that
        /// platform, but executed commands and environment variables still
        /// reflect the machine, so must not be used to inspect the platform.
        #[serde(default)]
        pub foreign: bool,

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
