mod describer;
mod detector;
mod downloader;
mod errors;
mod executor;
mod helpers;
mod installer;
mod manifest;
mod plugin;
mod resolver;
mod shimmer;
mod tool;
mod tools_config;
mod user_config;
mod verifier;

pub use async_trait::async_trait;
pub use describer::*;
pub use detector::*;
pub use downloader::*;
pub use errors::*;
pub use executor::*;
pub use helpers::*;
pub use installer::*;
pub use lenient_semver::Version;
pub use manifest::*;
pub use plugin::*;
pub use resolver::*;
pub use shimmer::*;
pub use starbase_styles::color;
pub use tool::*;
pub use tools_config::*;
pub use user_config::*;
pub use verifier::*;

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Proto {
    pub bin_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub home: PathBuf,
    pub root: PathBuf,
}

impl Proto {
    pub fn new() -> Result<Self, ProtoError> {
        let root = get_root()?;

        Ok(Proto {
            bin_dir: root.join("bin"),
            plugins_dir: root.join("plugins"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
            home: get_home_dir()?,
            root,
        })
    }

    pub fn new_testing(root: &Path) -> Self {
        Proto {
            bin_dir: root.join("bin"),
            plugins_dir: root.join("plugins"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
            home: root.join(".home"),
            root: root.to_owned(),
        }
    }

    pub fn from(root: &Path) -> Self {
        Proto {
            bin_dir: root.join("bin"),
            plugins_dir: root.join("plugins"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
            home: get_home_dir().expect("Missing home directory."),
            root: root.to_owned(),
        }
    }
}

impl AsRef<Proto> for Proto {
    fn as_ref(&self) -> &Proto {
        self
    }
}
