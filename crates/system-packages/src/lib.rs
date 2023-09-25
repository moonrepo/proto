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
pub enum Dependency {
    Single(String),
    SingleMap(HashMap<String, String>),
    Multiple(Vec<String>),
}

impl Default for Dependency {
    fn default() -> Dependency {
        Dependency::Single(String::new())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DependencyConfig {
    pub arch: Option<SystemArch>,
    pub args: Vec<String>,
    pub dep: Dependency,
    pub env: HashMap<String, String>,
    pub manager: Option<SystemPackageManager>,
    pub optional: bool,
    pub os: Option<SystemOS>,
    pub sudo: bool,
    pub version: Option<String>,
}

impl DependencyConfig {
    pub fn get_package_names(
        &self,
        os: &SystemOS,
        pm: &SystemPackageManager,
    ) -> Result<Vec<String>, Error> {
        match &self.dep {
            Dependency::Single(name) => Ok(vec![name.to_owned()]),
            Dependency::SingleMap(map) => map
                .get(&pm.to_string())
                .or_else(|| map.get(&os.to_string()))
                .or_else(|| map.get("*"))
                .map(|name| vec![name.to_owned()])
                .ok_or(Error::MissingName),
            Dependency::Multiple(list) => Ok(list.clone()),
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
    Names(Vec<String>),
    Config(DependencyConfig),
    Map(HashMap<String, String>),
}

impl SystemDependency {
    pub fn name(name: &str) -> SystemDependency {
        SystemDependency::Name(name.to_owned())
    }

    pub fn names<I, V>(names: I) -> SystemDependency
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        SystemDependency::Names(names.into_iter().map(|n| n.as_ref().to_owned()).collect())
    }

    pub fn for_arch(name: &str, arch: SystemArch) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            arch: Some(arch),
            dep: Dependency::Single(name.into()),
            ..DependencyConfig::default()
        })
    }

    pub fn for_os(name: &str, os: SystemOS) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            dep: Dependency::Single(name.into()),
            os: Some(os),
            ..DependencyConfig::default()
        })
    }

    pub fn for_os_arch(name: &str, os: SystemOS, arch: SystemArch) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            arch: Some(arch),
            dep: Dependency::Single(name.into()),
            os: Some(os),
            ..DependencyConfig::default()
        })
    }

    pub fn to_config(self) -> DependencyConfig {
        match self {
            Self::Name(name) => DependencyConfig {
                dep: Dependency::Single(name),
                ..DependencyConfig::default()
            },
            Self::Names(names) => DependencyConfig {
                dep: Dependency::Multiple(names),
                ..DependencyConfig::default()
            },
            Self::Map(map) => DependencyConfig {
                dep: Dependency::SingleMap(map),
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
