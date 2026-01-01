use crate::helpers::find_command_on_path;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env::consts;
use std::fmt;
use std::process::Command;

/// Architecture of the system environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "lowercase")]
pub enum SystemArch {
    X86,
    #[serde(alias = "x86_64")]
    X64,
    Arm,
    #[serde(alias = "aarch64")]
    Arm64,
    #[serde(alias = "loongarch64")]
    LongArm64,
    M68k,
    Mips,
    Mips64,
    Powerpc,
    Powerpc64,
    Riscv64,
    S390x,
    Sparc64,
}

impl SystemArch {
    /// Return an instance derived from [`std::env::costs::ARCH`].
    pub fn from_env() -> SystemArch {
        serde_json::from_value(Value::String(consts::ARCH.to_owned()))
            .expect("Unknown architecture!")
    }

    /// Convert to a [`std::env::costs::ARCH`] compatible string.
    pub fn to_rust_arch(&self) -> String {
        match self {
            Self::X64 => "x86_64".into(),
            Self::Arm64 => "aarch64".into(),
            Self::LongArm64 => "loongarch64".into(),
            _ => self.to_string(),
        }
    }
}

impl Default for SystemArch {
    #[cfg(target_arch = "wasm32")]
    fn default() -> Self {
        SystemArch::X64
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn default() -> Self {
        SystemArch::from_env()
    }
}

impl fmt::Display for SystemArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

/// Operating system of the current environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "lowercase")]
pub enum SystemOS {
    Android,
    Dragonfly,
    FreeBSD,
    IOS,
    Linux,
    #[serde(alias = "mac")]
    MacOS,
    NetBSD,
    OpenBSD,
    Solaris,
    Windows,
}

impl SystemOS {
    /// Return an instance derived from [`std::env::costs::OS`].
    pub fn from_env() -> SystemOS {
        serde_json::from_value(Value::String(consts::OS.to_owned()))
            .expect("Unknown operating system!")
    }

    /// Return either a Unix or Windows value based on the current native system.
    pub fn for_native<'value, T: AsRef<str> + ?Sized>(
        &self,
        unix: &'value T,
        windows: &'value T,
    ) -> &'value str {
        if self.is_windows() {
            windows.as_ref()
        } else {
            unix.as_ref()
        }
    }

    /// Return the provided name as a system formatted file name for executables.
    /// On Windows this will append an ".exe" extension. On Unix, no extension.
    pub fn get_exe_name(&self, name: impl AsRef<str>) -> String {
        self.get_file_name(name, "exe")
    }

    /// Return the provided file name formatted with the extension (without dot)
    /// when on Windows. On Unix, returns the name as-is.
    pub fn get_file_name(&self, name: impl AsRef<str>, windows_ext: impl AsRef<str>) -> String {
        let name = name.as_ref();
        let ext = windows_ext.as_ref();

        if self.is_windows() && !name.ends_with(ext) {
            format!("{name}.{ext}")
        } else {
            name.to_owned()
        }
    }

    /// Return true if in the BSD family.
    pub fn is_bsd(&self) -> bool {
        matches!(
            self,
            Self::Dragonfly | Self::FreeBSD | Self::NetBSD | Self::OpenBSD
        )
    }

    /// Return true if Linux.
    pub fn is_linux(&self) -> bool {
        matches!(self, Self::Linux)
    }

    /// Return true if MacOS.
    pub fn is_mac(&self) -> bool {
        matches!(self, Self::MacOS)
    }

    /// Return true if a Unix based OS.
    pub fn is_unix(&self) -> bool {
        self.is_bsd() || matches!(self, Self::Linux | Self::MacOS)
    }

    /// Return true if Windows.
    pub fn is_windows(&self) -> bool {
        matches!(self, Self::Windows)
    }

    /// Convert to a [`std::env::costs::OS`] compatible string.
    pub fn to_rust_os(&self) -> String {
        self.to_string()
    }
}

impl Default for SystemOS {
    #[cfg(target_arch = "wasm32")]
    fn default() -> Self {
        SystemOS::Linux
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn default() -> Self {
        SystemOS::from_env()
    }
}

impl fmt::Display for SystemOS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}

/// Libc being used in the system environment.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "lowercase")]
pub enum SystemLibc {
    Gnu,
    Musl,
    #[default]
    Unknown,
}

impl SystemLibc {
    /// Detect the libc type from the current system environment.
    pub fn detect(os: SystemOS) -> Self {
        match os {
            SystemOS::IOS | SystemOS::MacOS => Self::Gnu,
            SystemOS::Windows => Self::Unknown,
            _ => {
                if Self::is_musl() {
                    Self::Musl
                } else {
                    Self::Gnu
                }
            }
        }
    }

    /// Check if musl is available on the current machine, by running the
    /// `ldd --version` command, or the `uname` command. This will return false
    /// on systems that have neither of those commands.
    pub fn is_musl() -> bool {
        let mut command = if let Some(ldd_path) = find_command_on_path("ldd") {
            let mut cmd = Command::new(ldd_path);
            cmd.arg("--version");
            cmd
        } else if let Some(uname_path) = find_command_on_path("uname") {
            Command::new(uname_path)
        } else {
            return false;
        };

        if let Ok(result) = command.output() {
            let output = if result.status.success() {
                String::from_utf8_lossy(&result.stdout).to_lowercase()
            } else {
                // ldd on apline returns stderr with a 1 exit code
                String::from_utf8_lossy(&result.stderr).to_lowercase()
            };

            return output.contains("musl") || output.contains("alpine");
        }

        false
    }
}

impl fmt::Display for SystemLibc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}
