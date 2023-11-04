use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env::{self, consts};
use std::fmt;

/// Architecture of the host environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
    pub fn from_env() -> SystemArch {
        serde_json::from_value(Value::String(consts::ARCH.to_owned()))
            .expect("Unknown architecture!")
    }

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
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

/// Operating system of the host environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
    pub fn from_env() -> SystemOS {
        serde_json::from_value(Value::String(consts::OS.to_owned()))
            .expect("Unknown operating system!")
    }

    /// Return the provided name as a host formatted file name for executables.
    /// On Windows this will append an ".exe" extension. On Unix, no extension.
    pub fn get_exe_name(&self, name: impl AsRef<str>) -> String {
        self.get_file_name(name, "exe")
    }

    /// Return the provided file name formatted with the extension (without dot)
    /// when on Windows. On Unix, returns the name as-is.
    pub fn get_file_name(&self, name: impl AsRef<str>, windows_ext: impl AsRef<str>) -> String {
        if self.is_windows() {
            format!("{}.{}", name.as_ref(), windows_ext.as_ref())
        } else {
            name.as_ref().to_owned()
        }
    }

    /// Return either a Unix or Windows value based on the current host.
    pub fn get_native<'value, T: AsRef<str> + ?Sized>(
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

    pub fn is_bsd(&self) -> bool {
        matches!(
            self,
            Self::Dragonfly | Self::FreeBSD | Self::NetBSD | Self::OpenBSD
        )
    }

    pub fn is_linux(&self) -> bool {
        matches!(self, Self::Linux)
    }

    pub fn is_mac(&self) -> bool {
        matches!(self, Self::MacOS)
    }

    pub fn is_unix(&self) -> bool {
        self.is_bsd() || matches!(self, Self::Linux | Self::MacOS)
    }

    pub fn is_windows(&self) -> bool {
        matches!(self, Self::Windows)
    }

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
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[cfg(windows)]
pub fn is_command_on_path(name: &str) -> bool {
    let Ok(system_path) = env::var("PATH") else {
        return false;
    };
    let Ok(path_ext) = env::var("PATHEXT") else {
        return false;
    };
    let exts = path_ext.split(';').collect::<Vec<_>>();

    for path_dir in env::split_paths(&system_path) {
        for ext in &exts {
            if path_dir.join(format!("{name}{ext}")).exists() {
                return true;
            }
        }
    }

    false
}

#[cfg(not(windows))]
pub fn is_command_on_path(name: &str) -> bool {
    let Ok(system_path) = env::var("PATH") else {
        return false;
    };

    for path_dir in env::split_paths(&system_path) {
        if path_dir.join(name).exists() {
            return true;
        }
    }

    false
}
