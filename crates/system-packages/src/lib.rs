mod env;
mod error;
mod pm;
mod pm_vendor;

pub use env::*;
pub use error::*;
pub use pm::*;
pub use pm_vendor::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DependencyName {
    Name(String),
    Map(HashMap<String, String>),
}

impl Default for DependencyName {
    fn default() -> DependencyName {
        DependencyName::Name(String::new())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DependencyConfig {
    arch: Option<SystemArch>,
    args: Vec<String>,
    env: HashMap<String, String>,
    manager: Option<SystemPackageManager>,
    name: DependencyName,
    optional: bool,
    os: Option<SystemOS>,
    sudo: bool,
    version: Option<String>,
}

impl DependencyConfig {
    pub fn get_name(&self, os: &SystemOS, pm: &SystemPackageManager) -> Result<&str, Error> {
        match &self.name {
            DependencyName::Name(name) => Ok(name),
            DependencyName::Map(map) => map
                .get(&pm.to_string())
                .or_else(|| map.get(&os.to_string()))
                .or_else(|| map.get("*"))
                .map(|n| n.as_str())
                .ok_or(Error::MissingName),
        }
    }

    pub fn get_package_manager(&self) -> Result<PackageManager, Error> {
        if let Some(manager) = &self.manager {
            return Ok(PackageManager::from(*manager));
        }

        PackageManager::detect()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SystemDependency {
    Name(String),
    Config(DependencyConfig),
    Map(HashMap<String, String>),
}

impl SystemDependency {
    pub fn to_config(self) -> DependencyConfig {
        match self {
            Self::Name(name) => DependencyConfig {
                name: DependencyName::Name(name),
                ..DependencyConfig::default()
            },
            Self::Map(map) => DependencyConfig {
                name: DependencyName::Map(map),
                ..DependencyConfig::default()
            },
            Self::Config(config) => config,
        }
    }
}

pub fn resolve_dependencies(deps: Vec<SystemDependency>) -> Vec<DependencyConfig> {
    let os = SystemOS::from_env();
    let arch = SystemArch::from_env();
    let mut configs = vec![];

    for dep in deps {
        let config = dep.to_config();

        if config.os.as_ref().is_some_and(|o| o != &os) {
            continue;
        }

        if config.arch.as_ref().is_some_and(|a| a != &arch) {
            continue;
        }

        configs.push(config);
    }

    configs
}
