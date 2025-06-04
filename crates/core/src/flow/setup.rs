pub use super::setup_error::ProtoSetupError;
use crate::config::{PinLocation, ProtoConfig};
use crate::flow::install::{InstallOptions, ProtoInstallError};
use crate::layout::BinManager;
use crate::lockfile::LockfileRecord;
use crate::tool::Tool;
use crate::tool_manifest::ToolManifestVersion;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::{SyncManifestInput, SyncManifestOutput};
use starbase_utils::fs;
use std::collections::{BTreeMap, BTreeSet};
use tracing::{debug, instrument};

impl Tool {
    /// Return true if the tool has been setup (installed and binaries are located).
    #[instrument(skip(self))]
    pub async fn is_setup(&mut self, spec: &ToolSpec) -> Result<bool, ProtoSetupError> {
        self.resolve_version(spec, true).await?;

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

                // This conflicts with `proto run`...
                // self.locate_exe_file().await?;
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
        spec: &ToolSpec,
        options: InstallOptions,
    ) -> Result<Option<LockfileRecord>, ProtoSetupError> {
        let version = self.resolve_version(spec, false).await?;

        // Returns nothing if already installed
        let Some(record) = self.install(options).await? else {
            return Ok(self.inventory.get_locked_record(&version).cloned());
        };

        let default_version = self
            .metadata
            .default_version
            .clone()
            .unwrap_or_else(|| version.to_unresolved_spec());

        // Add version to manifest
        let manifest = &mut self.inventory.manifest;
        manifest.installed_versions.insert(version.clone());
        manifest.versions.insert(
            version.clone(),
            ToolManifestVersion {
                lock: Some(record.clone()),
                suffix: self.inventory.config.version_suffix.clone(),
                ..Default::default()
            },
        );
        manifest.save()?;

        // Pin the global version
        ProtoConfig::update(self.proto.get_config_dir(PinLocation::Global), |config| {
            config
                .versions
                .get_or_insert(Default::default())
                .entry(self.id.clone())
                .or_insert_with(|| ToolSpec::new_backend(default_version, self.backend));
        })?;

        // Allow plugins to override manifest
        self.sync_manifest().await?;

        // Create all the things
        self.generate_shims(false).await?;
        self.symlink_bins(true).await?;

        // Remove temp files
        self.cleanup().await?;

        Ok(Some(record))
    }

    /// Teardown the tool by uninstalling the current version, removing the version
    /// from the manifest, and cleaning up temporary files. Return true if the teardown occurred.
    #[instrument(skip_all)]
    pub async fn teardown(&mut self) -> Result<bool, ProtoSetupError> {
        self.cleanup().await?;

        if !self.uninstall().await? {
            return Ok(false);
        }

        let version = self.get_resolved_version();
        let mut bin_manager = BinManager::from_manifest(&self.inventory.manifest);

        let is_last_installed_version = self.inventory.manifest.installed_versions.len() == 1
            && self
                .inventory
                .manifest
                .installed_versions
                .contains(&version);

        // If no more versions in general, delete all
        if is_last_installed_version {
            for bin in self
                .resolve_bin_locations_with_manager(bin_manager, true)
                .await?
            {
                self.proto.store.unlink_bin(&bin.path)?;
            }

            for shim in self.resolve_shim_locations().await? {
                self.proto.store.remove_shim(&shim.path)?;
            }
        }
        // Otherwise, delete bins for this specific version
        else if bin_manager.remove_version(&version) {
            for bin in self
                .resolve_bin_locations_with_manager(bin_manager, false)
                .await?
            {
                self.proto.store.unlink_bin(&bin.path)?;
            }
        }

        // Unpin global version if a match
        ProtoConfig::update(self.proto.get_config_dir(PinLocation::Global), |config| {
            if let Some(versions) = &mut config.versions {
                if versions.get(&self.id).is_some_and(|v| v == &version) {
                    debug!("Unpinning global version");

                    versions.remove(&self.id);
                }
            }
        })?;

        // Remove version from manifest/lockfile
        // We must do this last because the location resolves above
        // require `installed_versions` to have values!
        let manifest = &mut self.inventory.manifest;
        manifest.installed_versions.remove(&version);
        manifest.versions.remove(&version);
        manifest.save()?;

        Ok(true)
    }

    /// Delete temporary files and downloads for the current version.
    #[instrument(skip_all)]
    pub async fn cleanup(&mut self) -> Result<(), ProtoSetupError> {
        debug!(
            tool = self.id.as_str(),
            "Cleaning up temporary files and downloads"
        );

        fs::remove_dir_all(self.get_temp_dir()).map_err(|error| {
            ProtoSetupError::Install(Box::new(ProtoInstallError::Fs(Box::new(error))))
        })?;

        Ok(())
    }

    /// Sync the local tool manifest with changes from the plugin.
    #[instrument(skip_all)]
    pub async fn sync_manifest(&mut self) -> Result<(), ProtoSetupError> {
        if !self.plugin.has_func("sync_manifest").await {
            return Ok(());
        }

        debug!(tool = self.id.as_str(), "Syncing manifest with changes");

        let output: SyncManifestOutput = self
            .plugin
            .call_func_with(
                "sync_manifest",
                SyncManifestInput {
                    context: self.create_context(),
                },
            )
            .await?;

        if output.skip_sync {
            return Ok(());
        }

        let mut modified = false;
        let manifest = &mut self.inventory.manifest;

        if let Some(versions) = output.versions {
            modified = true;

            let mut entries = BTreeMap::default();
            let mut installed = BTreeSet::default();

            for key in versions {
                let value = manifest.versions.get(&key).cloned().unwrap_or_default();

                installed.insert(key.clone());
                entries.insert(key, value);
            }

            manifest.versions = entries;
            manifest.installed_versions = installed;
        }

        if modified {
            manifest.save()?;
        }

        Ok(())
    }
}
