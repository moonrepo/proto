use crate::env::*;
use crate::error::Error;
use crate::pm::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DependencyName {
    Single(String),
    SingleMap(HashMap<String, String>),
    Multiple(Vec<String>),
}

impl Default for DependencyName {
    fn default() -> DependencyName {
        DependencyName::Single(String::new())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct DependencyConfig {
    pub arch: Option<SystemArch>,
    pub dep: DependencyName,
    pub manager: Option<SystemPackageManager>,
    // pub optional: bool,
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
            DependencyName::Single(name) => Ok(vec![name.to_owned()]),
            DependencyName::SingleMap(map) => map
                .get(&pm.to_string())
                .or_else(|| map.get(&os.to_string()))
                .or_else(|| map.get("*"))
                .map(|name| vec![name.to_owned()])
                .ok_or(Error::MissingName),
            DependencyName::Multiple(list) => Ok(list.clone()),
        }
    }
}

// This shape is what users configure.
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
            dep: DependencyName::Single(name.into()),
            ..DependencyConfig::default()
        })
    }

    pub fn for_os(name: &str, os: SystemOS) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            dep: DependencyName::Single(name.into()),
            os: Some(os),
            ..DependencyConfig::default()
        })
    }

    pub fn for_os_arch(name: &str, os: SystemOS, arch: SystemArch) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            arch: Some(arch),
            dep: DependencyName::Single(name.into()),
            os: Some(os),
            ..DependencyConfig::default()
        })
    }

    pub fn to_config(self) -> DependencyConfig {
        match self {
            Self::Name(name) => DependencyConfig {
                dep: DependencyName::Single(name),
                ..DependencyConfig::default()
            },
            Self::Names(names) => DependencyConfig {
                dep: DependencyName::Multiple(names),
                ..DependencyConfig::default()
            },
            Self::Map(map) => DependencyConfig {
                dep: DependencyName::SingleMap(map),
                ..DependencyConfig::default()
            },
            Self::Config(config) => config,
        }
    }
}
