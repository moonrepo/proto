use crate::error::PluginError;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;

/// Architecture of the host environment.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HostArch {
    X86,
    #[default]
    X64,
    Arm,
    Arm64,
    Mips,
    Mips64,
    Powerpc,
    Powerpc64,
    S390x,
}

impl Display for HostArch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

impl FromStr for HostArch {
    type Err = PluginError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86" => Ok(Self::X86),
            "x86_64" => Ok(Self::X64),
            "arm" => Ok(Self::Arm),
            "aarch64" => Ok(Self::Arm64),
            "mips" => Ok(Self::Mips),
            "mips64" => Ok(Self::Mips64),
            "powerpc" => Ok(Self::Powerpc),
            "powerpc64" => Ok(Self::Powerpc64),
            "s390x" => Ok(Self::S390x),
            arch => Err(PluginError::Message(format!(
                "Unsupported architecture {arch}."
            ))),
        }
    }
}

/// Operating system of the host environment.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HostOS {
    #[default]
    Linux,
    MacOS,
    FreeBSD,
    NetBSD,
    OpenBSD,
    Windows,
}

impl Display for HostOS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

impl FromStr for HostOS {
    type Err = PluginError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOS),
            "freebsd" => Ok(Self::FreeBSD),
            "netbsd" => Ok(Self::NetBSD),
            "openbsd" => Ok(Self::OpenBSD),
            "windows" => Ok(Self::Windows),
            os => Err(PluginError::Message(format!(
                "Unsupported operating system {os}."
            ))),
        }
    }
}
