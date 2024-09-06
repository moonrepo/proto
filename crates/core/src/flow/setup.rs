use crate::flow::install::InstallOptions;
use crate::proto_config::{PinType, ProtoConfig};
use crate::tool::Tool;
use crate::tool_manifest::ToolManifestVersion;
use proto_pdk_api::*;
use starbase_utils::fs;
use tracing::{debug, instrument};

impl Tool {
    /// Return true if the tool has been setup (installed and binaries are located).
    #[instrument(skip(self))]
    pub async fn is_setup(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
    ) -> miette::Result<bool> {
        self.resolve_version(initial_version, true).await?;

        let install_dir = self.get_product_dir();

        debug!(
            tool = self.id.as_str(),
            install_dir = ?install_dir,
            "Checking if tool is installed",
        );

        if self.is_installed() {
            debug!(
                tool = self.id.as_str(),
                install_dir = ?install_dir,
                "Tool has already been installed, locating binaries and shims",
            );

            if self.exe_file.is_none() {
                self.generate_shims(false).await?;
                self.symlink_bins(false).await?;
                self.locate_exe_file().await?;
            }

            return Ok(true);
        }

        debug!(tool = self.id.as_str(), "Tool has not been installed");

        Ok(false)
    }

    /// Setup the tool by resolving a semantic version, installing the tool,
    /// locating binaries, creating shims, and more.
    #[instrument(skip(self, options))]
    pub async fn setup(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
        options: InstallOptions,
    ) -> miette::Result<bool> {
        self.resolve_version(initial_version, false).await?;

        if !self.install(options).await? {
            return Ok(false);
        }

        self.generate_shims(false).await?;
        self.symlink_bins(false).await?;
        self.cleanup().await?;

        let version = self.get_resolved_version();
        let default_version = self
            .metadata
            .default_version
            .clone()
            .unwrap_or_else(|| version.to_unresolved_spec());

        // Add version to manifest
        let manifest = &mut self.inventory.manifest;
        manifest.installed_versions.insert(version.clone());
        manifest
            .versions
            .insert(version.clone(), ToolManifestVersion::default());
        manifest.save()?;

        // Pin the global version
        ProtoConfig::update(self.proto.get_config_dir(PinType::Global), |config| {
            config
                .versions
                .get_or_insert(Default::default())
                .entry(self.id.clone())
                .or_insert(default_version);
        })?;

        // Allow plugins to override manifest
        self.sync_manifest().await?;

        Ok(true)
    }

    /// Teardown the tool by uninstalling the current version, removing the version
    /// from the manifest, and cleaning up temporary files. Return true if the teardown occurred.
    #[instrument(skip_all)]
    pub async fn teardown(&mut self) -> miette::Result<bool> {
        self.cleanup().await?;

        if !self.uninstall().await? {
            return Ok(false);
        }

        let version = self.get_resolved_version();
        let mut removed_default_version = false;

        // Remove version from manifest
        let manifest = &mut self.inventory.manifest;
        manifest.installed_versions.remove(&version);
        manifest.versions.remove(&version);
        manifest.save()?;

        // Unpin global version if a match
        ProtoConfig::update(self.proto.get_config_dir(PinType::Global), |config| {
            if let Some(versions) = &mut config.versions {
                if versions.get(&self.id).is_some_and(|v| v == &version) {
                    debug!("Unpinning global version");

                    versions.remove(&self.id);
                    removed_default_version = true;
                }
            }
        })?;

        // If no more default version, delete the symlink,
        // otherwise the OS will throw errors for missing sources
        if removed_default_version || self.inventory.manifest.installed_versions.is_empty() {
            for bin in self.resolve_bin_locations().await? {
                self.proto.store.unlink_bin(&bin.path)?;
            }
        }

        // If no more versions in general, delete all shims
        if self.inventory.manifest.installed_versions.is_empty() {
            for shim in self.resolve_shim_locations().await? {
                self.proto.store.remove_shim(&shim.path)?;
            }
        }

        Ok(true)
    }

    /// Delete temporary files and downloads for the current version.
    #[instrument(skip_all)]
    pub async fn cleanup(&mut self) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Cleaning up temporary files and downloads"
        );

        fs::remove_dir_all(self.get_temp_dir())?;

        Ok(())
    }
}
