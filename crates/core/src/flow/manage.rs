pub use super::manage_error::ProtoManageError;
use crate::cfg;
use crate::config::{PinLocation, ProtoConfig};
use crate::flow::install::{InstallOptions, Installer, ProtoInstallError};
use crate::flow::link::Linker;
use crate::flow::locate::Locator;
use crate::flow::lock::Locker;
use crate::flow::resolve::Resolver;
use crate::layout::BinManager;
use crate::lockfile::LockRecord;
use crate::tool::Tool;
use crate::tool_manifest::ToolManifestVersion;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::{PluginFunction, SyncManifestInput, SyncManifestOutput};
use starbase_utils::fs;
use std::collections::{BTreeMap, BTreeSet};
use tracing::{debug, instrument};

/// Set up and tears down tools.
pub struct Manager<'tool> {
    tool: &'tool mut Tool,
}

impl<'tool> Manager<'tool> {
    pub fn new(tool: &'tool mut Tool) -> Self {
        Self { tool }
    }

    /// Setup the tool by resolving a semantic version, installing the tool,
    /// locating binaries, creating shims, and more.
    #[instrument(skip(self, options))]
    pub async fn install(
        &mut self,
        spec: &mut ToolSpec,
        options: InstallOptions,
    ) -> Result<Option<LockRecord>, ProtoManageError> {
        let version = Resolver::new(self.tool)
            .resolve_version(spec, false)
            .await?;

        let record = match Installer::new(self.tool, spec).install(options).await? {
            // Update lock record with resolved spec information
            Some(mut record) => {
                record.version = Some(version.clone());
                record.spec = Some(spec.req.clone());
                record
            }
            // Return an existing lock record if already installed
            None => {
                return Ok(Locker::new(self.tool)
                    .get_resolved_locked_record(spec)
                    .cloned());
            }
        };

        // Add record to lockfile
        if spec.update_lockfile {
            Locker::new(self.tool).insert_record_into_lockfile(&record)?;
        }

        // Add version to manifest
        self.tool.inventory.manifest.add_version(
            &version,
            ToolManifestVersion {
                lock: Some(record.for_manifest()),
                suffix: self.tool.inventory.config.version_suffix.clone(),
                ..Default::default()
            },
        );

        // Pin the global version
        ProtoConfig::update_document(self.tool.proto.get_config_dir(PinLocation::Global), |doc| {
            if !doc.contains_key(self.tool.get_id()) {
                doc[self.tool.context.as_str()] = cfg::value(
                    ToolSpec::new(
                        self.tool
                            .metadata
                            .default_version
                            .clone()
                            .unwrap_or_else(|| version.to_unresolved_spec()),
                    )
                    .to_string(),
                );
            }
        })?;

        // Link all the things
        let linker = Linker::new(self.tool, spec);
        linker.link_bins(false).await?;
        linker.link_shims(true).await?;

        // Remove temp files
        self.cleanup().await?;

        Ok(Some(record))
    }

    /// Teardown the tool by uninstalling the current version, removing the version
    /// from the manifest, and cleaning up temporary files. Return true if the teardown occurred.
    #[instrument(skip_all)]
    pub async fn uninstall(&mut self, spec: &mut ToolSpec) -> Result<bool, ProtoManageError> {
        self.cleanup().await?;

        let version = Resolver::new(self.tool)
            .resolve_version(spec, false)
            .await?;

        if !Installer::new(self.tool, spec).uninstall().await? {
            return Ok(false);
        }

        // Remove record from lockfile
        if spec.update_lockfile {
            Locker::new(self.tool).remove_version_from_lockfile(&version)?;
        }

        // Delete bins and shims
        let mut bin_manager = BinManager::from_manifest(&self.tool.inventory.manifest);
        let locator = Locator::new(self.tool, spec);
        let proto = &self.tool.proto;

        // If no more versions in general, delete all
        if self.tool.inventory.manifest.is_only_version(&version) {
            for bin in locator.locate_bins_with_manager(bin_manager, None).await? {
                proto.store.unlink_bin(&bin.path)?;
            }

            for shim in locator.locate_shims().await? {
                proto.store.remove_shim(&shim.path)?;
            }
        }
        // Otherwise, delete bins for this specific version
        else if bin_manager.remove_version(&version) {
            for bin in locator
                .locate_bins_with_manager(bin_manager, Some(&version))
                .await?
            {
                proto.store.unlink_bin(&bin.path)?;
            }
        }

        // Unpin global version if a match
        ProtoConfig::update_document(proto.get_config_dir(PinLocation::Global), |doc| {
            if doc
                .get(self.tool.context.as_str())
                .and_then(|item| item.as_str())
                .is_some_and(|v| version == v)
            {
                debug!("Unpinning global version");

                doc.as_table_mut().remove(self.tool.context.as_str());
            }
        })?;

        // We must do this last because the location resolves above
        // require `installed_versions` to have values!
        self.tool.inventory.manifest.remove_version(&version);

        Ok(true)
    }

    /// Delete temporary files and downloads for the current version.
    #[instrument(skip_all)]
    pub async fn cleanup(&self) -> Result<(), ProtoManageError> {
        debug!(
            tool = self.tool.context.as_str(),
            "Cleaning up temporary files and downloads"
        );

        fs::remove_dir_all(self.tool.get_temp_dir()).map_err(|error| {
            ProtoManageError::Install(Box::new(ProtoInstallError::Fs(Box::new(error))))
        })?;

        Ok(())
    }

    /// Sync the local tool manifest with changes from the plugin.
    #[instrument(skip_all)]
    pub async fn sync_manifest(self) -> Result<(), ProtoManageError> {
        if !self
            .tool
            .plugin
            .has_func(PluginFunction::SyncManifest)
            .await
        {
            self.tool.inventory.manifest.save()?;

            return Ok(());
        }

        debug!(
            tool = self.tool.context.as_str(),
            "Syncing manifest with changes"
        );

        let output: SyncManifestOutput = self
            .tool
            .plugin
            .call_func_with(
                PluginFunction::SyncManifest,
                SyncManifestInput {
                    context: self.tool.create_plugin_unresolved_context(),
                },
            )
            .await?;

        if !output.skip_sync
            && let Some(versions) = output.versions
        {
            let mut entries = BTreeMap::default();
            let mut installed = BTreeSet::default();

            for key in versions {
                let value = self
                    .tool
                    .inventory
                    .manifest
                    .versions
                    .get(&key)
                    .cloned()
                    .unwrap_or_default();

                installed.insert(key.clone());
                entries.insert(key, value);
            }

            self.tool.inventory.manifest.versions = entries;
            self.tool.inventory.manifest.installed_versions = installed;
        }

        self.tool.inventory.manifest.save()?;

        Ok(())
    }
}
