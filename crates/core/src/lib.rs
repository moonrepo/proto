pub mod color;
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
pub use resolver::*;
pub use shimmer::*;
pub use verifier::*;

use log::debug;
use std::fs;
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

#[async_trait::async_trait]
pub trait Tool<'tool>:
    Send
    + Sync
    + Describable<'tool>
    + Detector<'tool>
    + Resolvable<'tool>
    + Downloadable<'tool>
    + Verifiable<'tool>
    + Installable<'tool>
    + Executable<'tool>
    + Shimable<'tool>
{
    fn get_manifest_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(get_tools_dir()?
            .join(self.get_bin_name())
            .join(MANIFEST_NAME))
    }

    async fn before_setup(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    async fn setup(&mut self, initial_version: &str) -> Result<bool, ProtoError> {
        self.before_setup().await?;

        // Resolve a semantic version
        self.resolve_version(initial_version).await?;

        // Download the archive
        let download_path = self.get_download_path()?;

        self.download(&download_path, None).await?;

        // Verify the archive
        let checksum_path = self.get_checksum_path()?;

        self.download_checksum(&checksum_path, None).await?;
        self.verify_checksum(&checksum_path, &download_path).await?;

        // Install the tool
        let install_dir = self.get_install_dir()?;
        let installed = self.install(&install_dir, &download_path).await?;

        self.find_bin_path().await?;

        // Create shims after paths are found
        self.create_shims().await?;

        // Update the manifest
        Manifest::insert_version(self.get_manifest_path()?, self.get_resolved_version())?;

        self.after_setup().await?;

        Ok(installed)
    }

    async fn is_setup(&mut self, initial_version: &str) -> Result<bool, ProtoError> {
        self.resolve_version(initial_version).await?;

        let install_dir = self.get_install_dir()?;

        debug!(
            target: self.get_log_target(),
            "Checking for tool in {}",
            color::path(&install_dir),
        );

        if install_dir.exists() {
            self.find_bin_path().await?;

            let bin_path = {
                match self.get_bin_path() {
                    Ok(bin) => bin,
                    Err(_) => return Ok(false),
                }
            };

            if bin_path.exists() {
                debug!(
                    target: self.get_log_target(),
                    "Tool has already been installed at {}",
                    color::path(&install_dir)
                );

                self.create_shims().await?;

                return Ok(true);
            }
        } else {
            debug!(
                target: self.get_log_target(),
                "Tool has not been installed"
            );
        }

        Ok(false)
    }

    async fn after_setup(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<(), ProtoError> {
        debug!(
            target: self.get_log_target(),
            "Cleaning up temporary files and downloads"
        );

        let download_path = self.get_download_path()?;
        let checksum_path = self.get_checksum_path()?;

        if download_path.exists() {
            let _ = fs::remove_file(download_path);
        }

        if checksum_path.exists() {
            let _ = fs::remove_file(checksum_path);
        }

        Ok(())
    }

    async fn before_teardown(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    async fn teardown(&mut self) -> Result<(), ProtoError> {
        self.before_teardown().await?;

        self.cleanup().await?;

        let install_dir = self.get_install_dir()?;

        if install_dir.exists() {
            debug!(
                target: self.get_log_target(),
                "Deleting install directory {}",
                color::path(&install_dir)
            );

            fs::remove_dir_all(&install_dir)
                .map_err(|e| ProtoError::Fs(install_dir, e.to_string()))?;

            // Update the manifest
            Manifest::remove_version(self.get_manifest_path()?, self.get_resolved_version())?;
        }

        self.after_teardown().await?;

        Ok(())
    }

    async fn after_teardown(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }
}
