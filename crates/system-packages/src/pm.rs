use crate::error::Error;
use crate::pm_vendor::*;
use crate::{env::*, DependencyConfig};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
}

impl fmt::Display for SystemPackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

pub struct PackageManager {
    config: VendorConfig,
}

impl PackageManager {
    pub fn new(config: VendorConfig) -> Self {
        Self { config }
    }

    pub fn from(value: SystemPackageManager) -> Self {
        Self::new(match value {
            SystemPackageManager::Apk => apk(),
            SystemPackageManager::Apt => apt(),
            SystemPackageManager::Dnf => dnf(),
            SystemPackageManager::Pacman => pacman(),
            SystemPackageManager::Pkg => pkg(),
            SystemPackageManager::Pkgin => pkgin(),
            SystemPackageManager::Yum => yum(),
            SystemPackageManager::Brew => brew(),
            SystemPackageManager::Choco => choco(),
            SystemPackageManager::Scoop => scoop(),
        })
    }

    pub fn detect() -> Result<Self, Error> {
        #[cfg(target_os = "linux")]
        {
            let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();

            if let Some(id) = release.lines().find(|l| l.starts_with("ID=")) {
                return match id[3..].trim_matches('"') {
                    "debian" | "ubuntu" | "pop-os" | "deepin" | "elementary OS" | "kali"
                    | "linuxmint" => Ok(Self::from(SystemPackageManager::Apt)),
                    "arch" | "manjaro" => Ok(Self::from(SystemPackageManager::Pacman)),
                    "centos" | "redhat" | "rhel" => Ok(Self::from(SystemPackageManager::Yum)),
                    "fedora" => Ok(Self::from(SystemPackageManager::Dnf)),
                    "alpine" => Ok(Self::from(SystemPackageManager::Apk)),
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
            if is_command_on_path("pkg") {
                return Ok(Self::from(SystemPackageManager::Pkg));
            }

            if is_command_on_path("pkgin") {
                return Ok(Self::from(SystemPackageManager::Pkgin));
            }
        }

        #[cfg(target_os = "macos")]
        {
            if is_command_on_path("brew") {
                return Ok(Self::from(SystemPackageManager::Brew));
            }
        }

        #[cfg(target_os = "windows")]
        {
            if is_command_on_path("choco") {
                return Ok(Self::from(SystemPackageManager::Choco));
            }

            if is_command_on_path("scoop") {
                return Ok(Self::from(SystemPackageManager::Scoop));
            }
        }

        Err(Error::MissingPackageManager)
    }

    pub fn get_install_command(&self, dep_config: &DependencyConfig) -> Vec<String> {
        let mut args = vec![];

        for arg in &self.config.install_command {
            if arg == "$" {
                let dep = "TODO";

                if let Some(ver) = &dep_config.version {
                    match &self.config.version_arg {
                        VersionArgument::None => {}
                        VersionArgument::Inline(op) => {
                            args.push(format!("{dep}{op}{ver}"));
                        }
                        VersionArgument::Separate(opt) => {
                            args.push(dep.to_owned());
                            args.push(opt.to_owned());
                            args.push(ver.to_owned());
                        }
                    };
                } else {
                    args.push(dep.to_owned());
                }
            } else {
                args.push(arg.to_owned());
            }
        }

        args
    }

    pub fn get_update_index_command(&self) -> Option<Vec<String>> {
        self.config.update_index_command.clone()
    }
}
