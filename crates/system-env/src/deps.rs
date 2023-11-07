use crate::env::*;
use crate::error::Error;
use crate::pm::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A system dependency name in multiple formats.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(untagged)]
pub enum DependencyName {
    /// A single package by name.
    Single(String),

    /// A single package by name, but with different names (values)
    /// depending on operating system or package manager (keys).
    SingleMap(HashMap<String, String>),

    /// Multiple packages by name.
    Multiple(Vec<String>),
}

impl Default for DependencyName {
    fn default() -> DependencyName {
        DependencyName::Single(String::new())
    }
}

/// Configuration for one or many system dependencies (packages).
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(default)]
pub struct DependencyConfig {
    /// Only install on this architecture.
    pub arch: Option<SystemArch>,

    /// The dependency name or name(s) to install.
    pub dep: DependencyName,

    /// Only install with this package manager.
    pub manager: Option<SystemPackageManager>,

    /// Only install on this operating system.
    pub os: Option<SystemOS>,

    /// Install using sudo.
    pub sudo: bool,

    /// The version to install.
    pub version: Option<String>,
}

impl DependencyConfig {
    /// Get a list of package names for hte provided OS and package manager.
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

/// Represents a system dependency (one or many packages) to install.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(untagged)]
pub enum SystemDependency {
    /// A single package by name.
    Name(String),

    /// Multiple packages by name.
    Names(Vec<String>),

    /// Either a single or multiple package, defined as an
    /// explicit configuration object.
    Config(DependencyConfig),

    /// A single package by name, but with different names (values)
    /// depending on operating system or package manager (keys).
    Map(HashMap<String, String>),
}

impl SystemDependency {
    /// Create a single dependency by name.
    pub fn name(name: &str) -> SystemDependency {
        SystemDependency::Name(name.to_owned())
    }

    /// Create multiple dependencies by name.
    pub fn names<I, V>(names: I) -> SystemDependency
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        SystemDependency::Names(names.into_iter().map(|n| n.as_ref().to_owned()).collect())
    }

    /// Create a single dependency by name for the target architecture.
    pub fn for_arch(name: &str, arch: SystemArch) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            arch: Some(arch),
            dep: DependencyName::Single(name.into()),
            ..DependencyConfig::default()
        })
    }

    /// Create a single dependency by name for the target operating system.
    pub fn for_os(name: &str, os: SystemOS) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            dep: DependencyName::Single(name.into()),
            os: Some(os),
            ..DependencyConfig::default()
        })
    }

    /// Create a single dependency by name for the target operating system and architecture.
    pub fn for_os_arch(name: &str, os: SystemOS, arch: SystemArch) -> SystemDependency {
        SystemDependency::Config(DependencyConfig {
            arch: Some(arch),
            dep: DependencyName::Single(name.into()),
            os: Some(os),
            ..DependencyConfig::default()
        })
    }

    /// Convert and expand to a dependency configuration.
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
