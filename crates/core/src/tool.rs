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
use std::any::Any;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

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
    fn as_any(&self) -> &dyn Any;

    fn get_manifest(&self) -> Result<&Manifest, ProtoError>;

    fn get_manifest_mut(&mut self) -> Result<&mut Manifest, ProtoError>;

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
        let version = self.resolve_version(initial_version).await?;

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
            self.setup_shims(true).await?;

            // Update the manifest
            {
                let default_version = self.get_default_version().map(|v| v.to_owned());

                self.get_manifest_mut()?
                    .insert_version(&version, default_version)?;
            }

            self.after_setup().await?;

            return Ok(true);
        }

        Ok(false)
    }

    async fn setup_shims(&mut self, force: bool) -> Result<(), ProtoError> {
        let is_outdated = { self.get_manifest_mut()?.shim_version != SHIM_VERSION };

        if force || is_outdated {
            debug!("Creating shims as they either do not exist, or are outdated");

            let manifest = self.get_manifest_mut()?;
            manifest.shim_version = SHIM_VERSION;
            manifest.save()?;

            self.create_shims().await?;
        }

        Ok(())
    }

    async fn is_setup(&mut self, initial_version: &str) -> Result<bool, ProtoError> {
        self.resolve_version(initial_version).await?;

        let install_dir = self.get_install_dir()?;

        debug!(
            install_dir = %install_dir.display(),
            "Checking if tool is installed",
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
                    install_dir = %install_dir.display(),
                    "Tool has already been installed",
                );

                self.setup_shims(false).await?;

                return Ok(true);
            }
        } else {
            debug!("Tool has not been installed");
        }

        Ok(false)
    }

    async fn after_setup(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<(), ProtoError> {
        debug!("Cleaning up temporary files and downloads");

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
            let version = self.get_resolved_version().to_owned();

            self.get_manifest_mut()?.remove_version(&version)?;

            self.after_teardown().await?;

            return Ok(true);
        }

        Ok(false)
    }

    async fn after_teardown(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }
}

#[macro_export]
macro_rules! impl_tool {
    ($tool:ident) => {
        impl Tool<'_> for $tool {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn get_manifest(&self) -> Result<&Manifest, ProtoError> {
                self.manifest
                    .get_or_try_init(|| Manifest::load(self.get_manifest_path()))
            }

            fn get_manifest_mut(&mut self) -> Result<&mut Manifest, ProtoError> {
                {
                    // Ensure that the manifest has been initialized
                    self.get_manifest()?;
                }

                Ok(self.manifest.get_mut().unwrap())
            }

            fn get_tool_dir(&self) -> &Path {
                &self.base_dir
            }
        }
    };
}
