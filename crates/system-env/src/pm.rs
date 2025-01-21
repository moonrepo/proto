use crate::error::Error;
use crate::pm_vendor::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Package manager of the system environment.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(rename_all = "kebab-case")]
pub enum SystemPackageManager {
    // BSD
    Pkg,
    Pkgin,

    // Linux
    Apk,
    Apt,
    Dnf,
    Pacman,
    Yum,

    // MacOS
    #[serde(alias = "homebrew")]
    Brew,

    // Windows
    #[serde(alias = "chocolatey")]
    Choco,
    Scoop,

    // Used for name indexing
    #[serde(alias = "*")]
    All,
}

impl SystemPackageManager {
    /// Detect the package manager from the current system environment
    /// using the following rules:
    ///
    /// - On Linux, parses `/etc/os-release`.
    /// - On MacOS and BSD, checks for commands on `PATH`.
    /// - On Windows, checks for programs on `PATH`, using `PATHEXT`.
    pub fn detect() -> Result<Self, Error> {
        #[cfg(target_os = "linux")]
        {
            let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();

            if let Some(id) = release.lines().find(|l| l.starts_with("ID=")) {
                return match id[3..].trim_matches('"') {
                    "debian" | "ubuntu" | "pop-os" | "deepin" | "elementary OS" | "kali"
                    | "linuxmint" => Ok(SystemPackageManager::Apt),
                    "arch" | "manjaro" => Ok(SystemPackageManager::Pacman),
                    "centos" | "redhat" | "rhel" => Ok(SystemPackageManager::Yum),
                    "fedora" => Ok(SystemPackageManager::Dnf),
                    "alpine" => Ok(SystemPackageManager::Apk),
                    name => Err(Error::UnknownPackageManager(name.to_owned())),
                };
            }
        }

        #[cfg(any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            use crate::is_command_on_path;

            if is_command_on_path("pkg") {
                return Ok(SystemPackageManager::Pkg);
            }

            if is_command_on_path("pkgin") {
                return Ok(SystemPackageManager::Pkgin);
            }
        }

        #[cfg(target_os = "macos")]
        {
            use crate::is_command_on_path;

            if is_command_on_path("brew") {
                return Ok(SystemPackageManager::Brew);
            }
        }

        #[cfg(target_os = "windows")]
        {
            use crate::is_command_on_path;

            if is_command_on_path("choco") {
                return Ok(SystemPackageManager::Choco);
            }

            if is_command_on_path("scoop") {
                return Ok(SystemPackageManager::Scoop);
            }
        }

        Err(Error::MissingPackageManager)
    }

    /// Return vendor configuration for the current package manager.
    pub fn get_config(&self) -> PackageManagerConfig {
        match self {
            Self::Apk => apk(),
            Self::Apt => apt(),
            Self::Dnf => dnf(),
            Self::Pacman => pacman(),
            Self::Pkg => pkg(),
            Self::Pkgin => pkgin(),
            Self::Yum => yum(),
            Self::Brew => brew(),
            Self::Choco => choco(),
            Self::Scoop => scoop(),
            Self::All => unreachable!(),
        }
    }
}

impl fmt::Display for SystemPackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}
