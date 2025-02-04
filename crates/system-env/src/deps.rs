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
    /// depending on package manager (keys).
    SingleMap(HashMap<SystemPackageManager, String>),

    /// Multiple packages by name.
    Multiple(Vec<String>),

    /// Multiple packages by name, but with different names (values)
    /// depending on package manager (keys).
    MultipleMap(HashMap<SystemPackageManager, Vec<String>>),
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
    /// Get a list of package names for the provided OS and package manager.
    pub fn get_package_names(&self, pm: &SystemPackageManager) -> Result<Vec<String>, Error> {
        Ok(self
            .get_package_names_and_versions(pm)?
            .into_keys()
            .collect())
    }

    /// Get a list of package names and optional versions for the provided OS and package manager.
    pub fn get_package_names_and_versions(
        &self,
        pm: &SystemPackageManager,
    ) -> Result<HashMap<String, Option<String>>, Error> {
        let names = match &self.dep {
            DependencyName::Single(name) => vec![name.to_owned()],
            DependencyName::SingleMap(map) => map
                .get(pm)
                .or_else(|| map.get(&SystemPackageManager::All))
                .map(|name| vec![name.to_owned()])
                .ok_or(Error::MissingName)?,
            DependencyName::Multiple(list) => list.clone(),
            DependencyName::MultipleMap(map) => map
                .get(pm)
                .or_else(|| map.get(&SystemPackageManager::All))
                .cloned()
                .ok_or(Error::MissingName)?,
        };

        Ok(names
            .into_iter()
            .map(|name| {
                if name.contains('@') {
                    name.split_once('@')
                        .map(|(a, b)| (a.to_owned(), Some(b.to_owned())))
                        .unwrap()
                } else {
                    (name, self.version.clone())
                }
            })
            .collect())
    }

    /// Return true if the current config has the provided package name.
    pub fn has_name(&self, pm: &SystemPackageManager, name: &str) -> bool {
        self.get_package_names(pm)
            .map(|names| names.iter().any(|n| n == name))
            .unwrap_or(false)
    }
}

/// Represents a system dependency (one or many packages) to install.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(untagged)]
pub enum SystemDependency {
    /// A single package by name.
    Name(String),

    /// A single package by name, but with different names (values)
    /// depending on  package manager (keys).
    NameMap(HashMap<SystemPackageManager, String>),

    /// Multiple packages by name.
    Names(Vec<String>),

    /// Multiple packages by name, but with different names (values)
    /// depending on package manager (keys).
    NamesMap(HashMap<SystemPackageManager, Vec<String>>),

    /// Either a single or multiple package, defined as an
    /// explicit configuration object.
    Config(Box<DependencyConfig>),
}

impl SystemDependency {
    /// Create a single dependency by name.
    pub fn name(name: impl AsRef<str>) -> SystemDependency {
        SystemDependency::Name(name.as_ref().to_owned())
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
    pub fn for_arch(arch: SystemArch, name: impl AsRef<str>) -> SystemDependency {
        SystemDependency::Config(Box::new(DependencyConfig {
            arch: Some(arch),
            dep: DependencyName::Single(name.as_ref().into()),
            ..DependencyConfig::default()
        }))
    }

    /// Create a single dependency by name for the target operating system.
    pub fn for_os(os: SystemOS, name: impl AsRef<str>) -> SystemDependency {
        SystemDependency::Config(Box::new(DependencyConfig {
            dep: DependencyName::Single(name.as_ref().into()),
            os: Some(os),
            ..DependencyConfig::default()
        }))
    }

    /// Create a single dependency by name for the target operating system and architecture.
    pub fn for_os_arch(os: SystemOS, arch: SystemArch, name: impl AsRef<str>) -> SystemDependency {
        SystemDependency::Config(Box::new(DependencyConfig {
            arch: Some(arch),
            dep: DependencyName::Single(name.as_ref().into()),
            os: Some(os),
            ..DependencyConfig::default()
        }))
    }

    /// Create multiple dependencies by name for the target package manager.
    pub fn for_pm<I, V>(pm: SystemPackageManager, names: I) -> SystemDependency
    where
        I: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        SystemDependency::Config(Box::new(DependencyConfig {
            dep: DependencyName::Multiple(
                names.into_iter().map(|n| n.as_ref().to_owned()).collect(),
            ),
            manager: Some(pm),
            ..DependencyConfig::default()
        }))
    }

    /// Convert and expand to a dependency configuration.
    pub fn to_config(&self) -> DependencyConfig {
        match self {
            Self::Name(name) => DependencyConfig {
                dep: DependencyName::Single(name.to_owned()),
                ..DependencyConfig::default()
            },
            Self::NameMap(map) => DependencyConfig {
                dep: DependencyName::SingleMap(map.to_owned()),
                ..DependencyConfig::default()
            },
            Self::Names(names) => DependencyConfig {
                dep: DependencyName::Multiple(names.to_owned()),
                ..DependencyConfig::default()
            },
            Self::NamesMap(map) => DependencyConfig {
                dep: DependencyName::MultipleMap(map.to_owned()),
                ..DependencyConfig::default()
            },
            Self::Config(config) => (**config).to_owned(),
        }
    }
}
