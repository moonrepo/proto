use crate::color;
use crate::describer::*;
use crate::detector::*;
use crate::downloader::*;
use crate::errors::*;
use crate::executor::*;
use crate::installer::*;
use crate::manifest::*;
use crate::resolver::*;
use crate::shimmer::*;
use crate::verifier::*;
use log::debug;
use std::fs;
use std::path::{Path, PathBuf};

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
    fn get_manifest_path(&self) -> PathBuf {
        self.get_tool_dir().join(MANIFEST_NAME)
    }

    fn get_tool_dir(&self) -> &Path;

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

        if self.install(&install_dir, &download_path).await? {
            self.find_bin_path().await?;

            // Create shims after paths are found
            self.create_shims().await?;

            // Update the manifest
            Manifest::insert_version(
                self.get_manifest_path(),
                self.get_resolved_version(),
                self.get_default_version(),
            )?;

            self.after_setup().await?;

            return Ok(true);
        }

        Ok(false)
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

    async fn teardown(&mut self) -> Result<bool, ProtoError> {
        self.before_teardown().await?;

        self.cleanup().await?;

        let install_dir = self.get_install_dir()?;

        if self.uninstall(&install_dir).await? {
            Manifest::remove_version(self.get_manifest_path(), self.get_resolved_version())?;

            self.after_teardown().await?;

            return Ok(true);
        }

        Ok(false)
    }

    async fn after_teardown(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }
}
