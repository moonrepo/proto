use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env::{self, consts};
use std::fmt;

/// Architecture of the host environment.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemArch {
    X86,
    #[default]
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

impl fmt::Display for SystemArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

/// Operating system of the host environment.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemOS {
    Android,
    Dragonfly,
    FreeBSD,
    IOS,
    #[default]
    Linux,
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

    pub fn is_bsd(&self) -> bool {
        matches!(
            self,
            Self::Dragonfly | Self::FreeBSD | Self::NetBSD | Self::OpenBSD
        )
    }

    pub fn is_unix(&self) -> bool {
        self.is_bsd() || matches!(self, Self::Linux | Self::MacOS)
    }

    pub fn to_rust_os(&self) -> String {
        self.to_string()
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
