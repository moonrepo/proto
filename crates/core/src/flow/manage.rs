pub use super::manage_error::ProtoManageError;
use crate::flow::install::{InstallOptions, Installer};
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

#[allow(clippy::large_enum_variant)]
enum InstallOutcome {
    AlreadyInstalled,
    InstalledConcurrently,
    Installed(LockRecord),
}

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
    ///
    /// Concurrent installs of the same version, from any process, are
    /// serialized through a lock on the version-keyed temp directory, and
    /// only the first will do the actual work.
    #[instrument(skip(self, options))]
    pub async fn install(
        &mut self,
        spec: &mut ToolSpec,
        options: InstallOptions,
    ) -> Result<Option<LockRecord>, ProtoManageError> {
        let timer = self.tool.proto.create_metric();
        let strategy = install_strategy_name(&options.strategy);
        let mut cache = "unknown";

        // Lock the version-keyed temporary directory instead of the
        // install directory, because the latter needs to be clean for
        // "build from source", and the `.lock` file breaks that contract
        let temp_dir = self.tool.get_version_temp_dir(spec);
        let mut install_lock = fs::lock_directory(&temp_dir)?;

        let result = async {
            match self.do_install(spec, options).await? {
                InstallOutcome::AlreadyInstalled | InstallOutcome::InstalledConcurrently => {
                    cache = "hit";
                    self.post_install(spec, None).await?;

                    Ok(None)
                }
                InstallOutcome::Installed(record) => {
                    cache = "miss";
                    self.post_install(spec, Some(&record)).await?;

                    Ok(Some(record))
                }
            }
        }
        .await;

        // Unlock and then remove the version-keyed temp directory. Removal must
        // happen after unlocking, as some platforms (Windows) cannot delete a
        // directory containing an open lock file.
        install_lock.unlock()?;
        let _ = fs::remove_dir_all(temp_dir);

        timer.record_tool_install(&self.tool.context, strategy, cache, result)
    }

    async fn do_install(
        &mut self,
        spec: &mut ToolSpec,
        options: InstallOptions,
    ) -> Result<InstallOutcome, ProtoManageError> {
        let version = Resolver::resolve(self.tool, spec, false).await?;

        if self.tool.is_installed(spec) && !options.force {
            return Ok(InstallOutcome::AlreadyInstalled);
        }

        // While we were waiting on the lock, another process may have
        // installed this version, so merge the latest manifest from
        // disk and check again
        if self.was_installed_concurrently(spec, &options).await? {
            return Ok(InstallOutcome::InstalledConcurrently);
        }

        let record = match Installer::new(self.tool, spec).install(options).await {
            // Update lock record with resolved spec information
            Ok(Some(mut record)) => {
                record.version = Some(version.clone());
                record.spec = Some(spec.req.clone());
                record
            }
            // Return an existing lock record if already installed
            Ok(None) => {
                return Ok(InstallOutcome::AlreadyInstalled);
            }
            // Clean up our partial install if it failed. This can never
            // delete a live install: reaching the installer requires the
            // version to not be installed, or `force` to be set, in which
            // case the directory contents are already suspect
            Err(error) => {
                debug!(
                    tool = self.tool.context.as_str(),
                    install_dir = ?self.tool.get_product_dir(spec),
                    "Failed to install tool, cleaning up",
                );

                let _ = fs::remove_dir_all(self.tool.get_product_dir(spec));

                return Err(error.into());
            }
        };

        Ok(InstallOutcome::Installed(record))
    }

    #[instrument(skip(self))]
    async fn post_install(
        &mut self,
        spec: &mut ToolSpec,
        record: Option<&LockRecord>,
    ) -> Result<(), ProtoManageError> {
        if let Some(record) = record {
            // Add record to lockfile
            if spec.update_lockfile {
                Locker::new(self.tool).insert_record_into_lockfile(record)?;
            }

            // Add version to manifest and persist it *before* releasing the
            // lock, so that other processes waiting on the lock immediately
            // see the completed install once they acquire it
            self.tool.inventory.manifest.add_version(
                record.version.as_ref().unwrap(),
                ToolManifestVersion {
                    lock: Some(record.for_manifest()),
                    suffix: self.tool.inventory.config.version_suffix.clone(),
                    ..Default::default()
                },
            );

            self.tool.inventory.manifest.save()?;
        }

        // Link all the things
        Linker::link(self.tool, spec, record.is_some()).await?;

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

            // Remove records from all lockfiles, as the version
            // may be tracked by multiple configs
            if spec.update_lockfile {
                for file in self.tool.proto.load_file_manager()?.get_config_files() {
                    if file.locked {
                        Locker::for_config(self.tool, &file.path)
                            .remove_version_from_lockfile(&version)?;
                    }
                }
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

    /// Delete temporary files and downloads for this tool. Directories that
    /// are locked by another process (an install currently in progress) are
    /// skipped, so that we don't delete its lock file or in-flight downloads.
    #[instrument(skip(self))]
    pub async fn cleanup(&self) -> Result<(), ProtoManageError> {
        debug!(
            tool = self.tool.context.as_str(),
            "Cleaning up temporary files and downloads"
        );

        let temp_dir = self.tool.get_temp_dir();

        if !temp_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(temp_dir)? {
            let path = entry.path();

            if path.is_dir() {
                if fs::is_dir_locked(&path) {
                    debug!(
                        tool = self.tool.context.as_str(),
                        dir = ?path,
                        "Skipping temporary directory, an install is currently in progress"
                    );

                    continue;
                }

                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }

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

    async fn was_installed_concurrently(
        &mut self,
        spec: &ToolSpec,
        options: &InstallOptions,
    ) -> Result<bool, ProtoManageError> {
        self.tool.inventory.manifest.reload_from_disk()?;

        if self.tool.is_installed(spec) && !options.force {
            debug!(
                tool = self.tool.context.as_str(),
                "Tool was installed by another process while waiting on the install lock, continuing"
            );

            return Ok(true);
        }

        Ok(false)
    }
}

fn install_strategy_name(strategy: &InstallStrategy) -> &'static str {
    match strategy {
        InstallStrategy::BuildFromSource => "build-from-source",
        InstallStrategy::DownloadPrebuilt => "download-prebuilt",
    }
}
