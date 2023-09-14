mod env;

pub use env::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SystemPackageManager {
    // Linux
    Apk,
    Apt,
    Dnf,
    Pacman,
    Yum,

    // MacOS
    Brew,

    // Windows
    Choco,
    Scoop,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct DependencyConfig {
    arch: Option<Arch>,
    args: Vec<String>,
    env: HashMap<String, String>,
    manager: Option<SystemPackageManager>,
    name: String,
    optional: bool,
    os: Option<OS>,
    version: Option<String>,
}

pub enum SystemDependency {
    Name(String),
    Config(DependencyConfig),
}

impl SystemDependency {
    pub fn to_config(self) -> DependencyConfig {
        match self {
            Self::Name(name) => DependencyConfig {
                name,
                ..DependencyConfig::default()
            },
            Self::Config(config) => config,
        }
    }
}
