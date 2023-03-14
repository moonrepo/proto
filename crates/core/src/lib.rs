pub mod color;
mod config;
mod describer;
mod detector;
mod downloader;
mod errors;
mod executor;
mod helpers;
mod installer;
mod manifest;
mod resolver;
mod shimmer;
mod tool;
mod verifier;

pub use async_trait::async_trait;
pub use config::*;
pub use describer::*;
pub use detector::*;
pub use downloader::*;
pub use errors::*;
pub use executor::*;
pub use helpers::*;
pub use installer::*;
pub use lenient_semver::Version;
pub use manifest::*;
pub use resolver::*;
pub use shimmer::*;
pub use tool::*;
pub use verifier::*;

use std::path::{Path, PathBuf};

pub struct Proto {
    pub bin_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub tools_dir: PathBuf,
}

impl Proto {
    pub fn new() -> Result<Self, ProtoError> {
        let root = get_root()?;

        Ok(Proto {
            bin_dir: root.join("bin"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
        })
    }

    pub fn from(root: &Path) -> Self {
        Proto {
            bin_dir: root.join("bin"),
            temp_dir: root.join("temp"),
            tools_dir: root.join("tools"),
        }
    }
}
