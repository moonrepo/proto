pub use super::manage_error::ProtoManageError;
use crate::flow::install::{InstallOptions, Installer, ProtoInstallError};
use crate::flow::link::Linker;
use crate::flow::lock::Locker;
use crate::flow::resolve::Resolver;
use crate::lockfile::LockRecord;
use crate::telemetry::cache_status;
use crate::tool::Tool;
use crate::tool_manifest::ToolManifestVersion;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::{InstallStrategy, PluginFunction, SyncManifestInput, SyncManifestOutput};
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
        let timer = self.tool.proto.create_metric();
        let strategy = install_strategy_name(&options.strategy);
        let mut cache = "unknown";

        let result = async {
            let version = Resolver::resolve(self.tool, spec, false).await?;
            let cache_hit = self.tool.is_installed(spec) && !options.force;
            cache = cache_status(cache_hit);

            let record = match Installer::new(self.tool, spec).install(options).await? {
                // Update lock record with resolved spec information
                Some(mut record) => {
                    record.version = Some(version.clone());
                    record.spec = Some(spec.req.clone());
                    record
                }
                // Return an existing lock record if already installed
                None => {
                    self.post_install(spec, false).await?;

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

            self.post_install(spec, true).await?;

            Ok(Some(record))
        }
        .await;

        timer.record_tool_install(&self.tool.context, strategy, cache, result)
    }

    #[instrument(skip(self))]
    async fn post_install(&self, spec: &mut ToolSpec, force: bool) -> Result<(), ProtoManageError> {
        // Link all the things
        Linker::link(self.tool, spec, force).await?;

        // Remove temp files
        self.cleanup().await?;

        Ok(())
    }

    /// Teardown the tool by uninstalling the current version, removing the version
    /// from the manifest, and cleaning up temporary files. Return true if the teardown occurred.
    #[instrument(skip(self))]
    pub async fn uninstall(&mut self, spec: &mut ToolSpec) -> Result<bool, ProtoManageError> {
        let timer = self.tool.proto.create_metric();
        let mut cache = "unknown";

        let result = async {
            self.cleanup().await?;

            let version = Resolver::resolve(self.tool, spec, false).await?;
            cache = cache_status(self.tool.is_installed(spec));

            if !Installer::new(self.tool, spec).uninstall().await? {
                return Ok(false);
            }

            // Remove record from lockfile
            if spec.update_lockfile {
                Locker::new(self.tool).remove_version_from_lockfile(&version)?;
            }

            // Delete bins and shims
            let linker = Linker::new(self.tool, spec)?;

            // If no more versions in general, delete everything. Otherwise,
            // reconcile the bins for just this version: orphaned bins are
            // removed and shared bins are re-pointed to the next highest version.
            if self.tool.inventory.manifest.installed_versions.is_empty()
                || self.tool.inventory.manifest.is_only_version(&version)
            {
                linker.unlink_bins().await?;
                linker.unlink_shims().await?;
            } else {
                linker.unlink_bins_by_version(&version).await?;
            }

            // We must do this last because the location resolves above
            // require `installed_versions` to have values!
            self.tool.inventory.manifest.remove_version(&version);

            Ok(true)
        }
        .await;

        timer.record_tool_uninstall(&self.tool.context, "version", cache, result)
    }

    /// Delete temporary files and downloads for the current version.
    #[instrument(skip(self))]
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
    #[instrument(skip(self))]
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

fn install_strategy_name(strategy: &InstallStrategy) -> &'static str {
    match strategy {
        InstallStrategy::BuildFromSource => "build-from-source",
        InstallStrategy::DownloadPrebuilt => "download-prebuilt",
    }
}
