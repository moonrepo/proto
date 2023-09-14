use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env::consts;
use std::fmt;

/// Architecture of the host environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
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

impl Arch {
    pub fn from_env() -> Arch {
        serde_json::from_value(Value::String(consts::ARCH.to_owned()))
            .expect("Unknown architecture!")
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

/// Operating system of the host environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OS {
    Android,
    Dragonfly,
    FreeBSD,
    IOS,
    Linux,
    MacOS,
    NetBSD,
    OpenBSD,
    Solaris,
    Windows,
}

impl OS {
    pub fn from_env() -> OS {
        serde_json::from_value(Value::String(consts::OS.to_owned()))
            .expect("Unknown operating system!")
    }

    pub fn is_bsd(&self) -> bool {
        matches!(
            self,
            Self::Dragonfly | Self::FreeBSD | Self::NetBSD | Self::OpenBSD
        )
    }

    pub fn is_linux(&self) -> bool {
        self.is_bsd() || matches!(self, Self::Linux)
    }
}

impl fmt::Display for OS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}
