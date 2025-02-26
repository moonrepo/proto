use crate::config::{PinLocation, ProtoConfig};
use crate::flow::install::InstallOptions;
use crate::layout::BinManager;
use crate::tool::Tool;
use crate::tool_manifest::ToolManifestVersion;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::*;
use starbase_utils::fs;
use tracing::{debug, instrument};

impl Tool {
    #[instrument(skip(self))]
    pub async fn is_setup_with_spec(&mut self, spec: &ToolSpec) -> miette::Result<bool> {
        self.resolve_version_with_spec(spec, true).await?;
        self.is_setup(&spec.req).await
    }

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

                // This conflicts with `proto run`...
                // self.locate_exe_file().await?;
            }

            return Ok(true);
        }

        debug!(tool = self.id.as_str(), "Tool has not been installed");

        Ok(false)
    }

    #[instrument(skip(self, options))]
    pub async fn setup_with_spec(
        &mut self,
        spec: &ToolSpec,
        options: InstallOptions,
    ) -> miette::Result<bool> {
        self.resolve_version_with_spec(spec, false).await?;
        self.setup(&spec.req, options).await
    }

    /// Setup the tool by resolving a semantic version, installing the tool,
    /// locating binaries, creating shims, and more.
    #[instrument(skip(self, options))]
    pub async fn setup(
        &mut self,
        initial_version: &UnresolvedVersionSpec,
        options: InstallOptions,
    ) -> miette::Result<bool> {
        let version = self.resolve_version(initial_version, false).await?;

        if !self.install(options).await? {
            return Ok(false);
        }

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
                backend: self.backend,
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

        // Remove version from manifest
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
    pub async fn cleanup(&mut self) -> miette::Result<()> {
        debug!(
            tool = self.id.as_str(),
            "Cleaning up temporary files and downloads"
        );

        fs::remove_dir_all(self.get_temp_dir())?;

        Ok(())
    }
}
