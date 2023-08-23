use crate::error::PluginError;
use crate::json_struct;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use warpgate_api::VirtualPath;

/// Architecture of the host environment.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

impl HostArch {
    pub fn to_rust_arch(&self) -> String {
        match self {
            Self::X64 => "x86_64".into(),
            Self::Arm64 => "aarch64".into(),
            _ => self.to_string(),
        }
    }
}

impl fmt::Display for HostArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

impl HostOS {
    pub fn is_bsd(&self) -> bool {
        matches!(self, Self::FreeBSD | Self::NetBSD | Self::OpenBSD)
    }

    pub fn is_linux(&self) -> bool {
        !matches!(self, Self::MacOS | Self::Windows)
    }

    pub fn to_rust_os(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for HostOS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

json_struct!(
    /// Information about the host environment (the current runtime).
    pub struct HostEnvironment {
        pub arch: HostArch,
        pub os: HostOS,
        pub home_dir: VirtualPath,
        pub proto_dir: VirtualPath,
    }
);

json_struct!(
    /// The current user's proto configuration.
    pub struct UserConfigSettings {
        pub auto_clean: bool,
        pub auto_install: bool,
        pub node_intercept_globals: bool,
    }
);
