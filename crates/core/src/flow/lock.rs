pub use super::lock_error::ProtoLockError;
use crate::ProtoEnvironment;
use crate::lockfile::{LockRecord, ProtoLock};
use crate::tool::Tool;
use crate::tool_context::ToolContext;
use crate::tool_spec::ToolSpec;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, instrument};
use version_spec::{UnresolvedVersionSpec, VersionSpec};

// [x] install many
//      [x] resolve version from lockfile
//      [x] validate lock record
//      [x] create lockfile if it does not exist
//      [x] error if spec/req is not found in lockfile
//      [ ] frozen lockfiles
//      [x] orphan pruning
// [x] install one
//      [x] resolve version from lockfile
//      [x] validate lock record
//      [ ] frozen lockfiles
//      [x] orphan pruning
// [x] install one version
//      [x] don't resolve version from lockfile
//      [x] validate lock record
//      [ ] frozen lockfiles
//      [x] orphan pruning
// [x] uninstall
//      [x] remove from lockfile
//      [x] orphan pruning
// [x] outdated
//      [x] add locked label to table
//      [x] integrate with --update
// [x] run
//      [x] resolve version from lockfile
// [x] status
//      [x] add locked label to table

/// Manages records in a lockfile.
pub struct Locker<'tool> {
    tool: &'tool Tool,
    config_path: Option<PathBuf>,
}

impl<'tool> Locker<'tool> {
    /// Create a locker for the lockfile that applies to the tool, which is
    /// owned by the closest config that defines a version for it.
    pub fn new(tool: &'tool Tool) -> Self {
        Self {
            tool,
            config_path: None,
        }
    }

    /// Create a locker for the lockfile owned by the config at the provided
    /// path, regardless of which config defines a version for the tool.
    /// Useful when a config has been modified and its lockfile must be kept
    /// in sync. Does nothing if the config has not enabled a lockfile.
    pub fn for_config(tool: &'tool Tool, config_path: impl AsRef<Path>) -> Self {
        Self {
            tool,
            config_path: Some(config_path.as_ref().to_path_buf()),
        }
    }

    fn load_lock(&self) -> Result<Option<RwLockReadGuard<'_, ProtoLock>>, ProtoLockError> {
        let proto = &self.tool.proto;

        Ok(match &self.config_path {
            Some(config_path) => proto.load_file_manager()?.get_lock(config_path)?,
            None => proto.load_lock(&self.tool.context)?,
        })
    }

    fn load_lock_mut(&self) -> Result<Option<RwLockWriteGuard<'_, ProtoLock>>, ProtoLockError> {
        let proto = &self.tool.proto;

        Ok(match &self.config_path {
            Some(config_path) => proto.load_file_manager()?.get_lock_mut(config_path)?,
            None => proto.load_lock_mut(&self.tool.context)?,
        })
    }

    /// Remove orphaned records from all loaded lockfiles. A record is
    /// orphaned when its tool has a version defined in the lockfile's
    /// sibling configs, but the record's spec no longer matches any of
    /// the configured specs, which is typically caused by a config being
    /// modified outside of proto. Records for tools that are not defined
    /// in a sibling config are ad-hoc installs, and are never pruned.
    ///
    /// Specs that are actively being installed can be provided, so that
    /// records the current command depends on are never pruned.
    ///
    /// This should only be triggered from explicit install and uninstall
    /// flows, as other flows should not modify the lockfile.
    #[instrument(skip(proto))]
    pub fn prune_orphaned_records(
        proto: &ProtoEnvironment,
        installing: &BTreeMap<ToolContext, UnresolvedVersionSpec>,
    ) -> Result<usize, ProtoLockError> {
        let manager = proto.load_file_manager()?;
        let mut pruned = 0;

        for entry in &manager.entries {
            if !entry.locked {
                continue;
            }

            // Gather specs defined in sibling configs, as only these
            // configs dictate which records in the lockfile are used
            let mut config_specs: BTreeMap<ToolContext, BTreeSet<UnresolvedVersionSpec>> =
                BTreeMap::default();

            for file in &entry.configs {
                if let Some(versions) = &file.config.versions {
                    for (context, spec) in versions {
                        config_specs
                            .entry(context.to_owned())
                            .or_default()
                            .insert(spec.req.clone());
                    }
                }
            }

            // If nothing is configured, all records are ad-hoc installs
            if config_specs.is_empty() {
                continue;
            }

            // Treat specs that are actively being installed as if they
            // were configured, but only when this lockfile owns the
            // tool's records, so that in-flight records are never pruned
            for (context, spec) in installing {
                if let Some(specs) = config_specs.get_mut(context)
                    && manager
                        .get_locked_dir(context)
                        .is_some_and(|dir| dir == entry.path)
                {
                    specs.insert(spec.to_owned());
                }
            }

            let Some(mut lock) = manager.get_lock_mut(&entry.path)? else {
                continue;
            };

            let count = lock.prune_orphaned_records(&config_specs);

            if count > 0 {
                debug!(
                    file = ?lock.path,
                    "Pruned {count} orphaned record(s) from lock file",
                );

                lock.sort_records();
                lock.save()?;

                pruned += count;
            }
        }

        Ok(pruned)
    }

    /// Get all resolved versions that have a record in the lockfile,
    /// scoped to the current operating system and architecture.
    pub fn get_locked_versions(&self) -> Result<BTreeSet<VersionSpec>, ProtoLockError> {
        let proto = &self.tool.proto;
        let mut versions = BTreeSet::default();

        let Some(lock) = self.load_lock()? else {
            return Ok(versions);
        };

        if let Some(records) = lock.tools.get(self.tool.get_id()) {
            for record in records {
                if record.backend.as_ref() == self.tool.context.backend.as_ref()
                    && record.os.is_none_or(|os| os == proto.os)
                    && record.arch.is_none_or(|arch| arch == proto.arch)
                    && let Some(version) = &record.version
                {
                    versions.insert(version.to_owned());
                }
            }
        }

        Ok(versions)
    }

    pub fn get_resolved_locked_record<'a>(&'a self, spec: &'a ToolSpec) -> Option<&'a LockRecord> {
        // From lockfile (after being resolved)
        if let Some(record) = &spec.version_locked {
            return Some(record);
        }

        // From manifest
        if let Some(version) = &spec.version {
            return self.tool.inventory.get_locked_record(version);
        }

        None
    }

    #[instrument(skip(self))]
    pub fn insert_record_into_lockfile(&self, record: &LockRecord) -> Result<(), ProtoLockError> {
        if self.tool.metadata.lock_options.no_record {
            return Ok(());
        }

        let proto = &self.tool.proto;

        let Some(mut lock) = self.load_lock_mut()? else {
            return Ok(());
        };

        let record = record.for_lockfile();
        let records = lock.tools.entry(self.tool.get_id().to_owned()).or_default();

        // Find an existing record with the same spec
        match records
            .iter_mut()
            .find(|existing| existing.is_match(&record, &self.tool.metadata.lock_options))
        {
            Some(existing) => {
                // If the new record has a higher version,
                // we should replace the existing record with it
                if let (Some(record_version), Some(existing_version)) =
                    (&record.version, &existing.version)
                    && record_version >= existing_version
                {
                    *existing = record;
                }

                // Backwards compatibility for records without an os/arch
                if self.tool.metadata.lock_options.ignore_os_arch {
                    existing.os = None;
                    existing.arch = None;
                } else {
                    existing.os.get_or_insert(proto.os);
                    existing.arch.get_or_insert(proto.arch);
                }
            }
            None => {
                records.push(record);
            }
        };

        lock.sort_records();
        lock.save()?;

        Ok(())
    }

    pub fn remove_from_lockfile(&self) -> Result<(), ProtoLockError> {
        let Some(mut lock) = self.load_lock_mut()? else {
            return Ok(());
        };

        // Nothing was removed, so avoid saving
        if lock.tools.remove(self.tool.get_id()).is_none() {
            return Ok(());
        }

        lock.save()?;

        Ok(())
    }

    pub fn remove_version_from_lockfile(
        &self,
        version: &VersionSpec,
    ) -> Result<(), ProtoLockError> {
        let proto = &self.tool.proto;

        let Some(mut lock) = self.load_lock_mut()? else {
            return Ok(());
        };

        let Some(records) = lock.tools.get_mut(self.tool.get_id()) else {
            return Ok(());
        };

        let spec = version.to_unresolved_spec();
        let count = records.len();

        records.retain(|record| {
            let matched = record.is_match_with(
                self.tool.context.backend.as_ref(),
                Some(&spec),
                Some(&proto.os),
                Some(&proto.arch),
                &self.tool.metadata.lock_options,
            );

            !(matched && record.version.as_ref().is_some_and(|ver| ver == version))
        });

        // Nothing was removed, so avoid saving
        if records.len() == count && !records.is_empty() {
            return Ok(());
        }

        if records.is_empty() {
            lock.tools.remove(self.tool.get_id());
        }

        lock.sort_records();
        lock.save()?;

        Ok(())
    }

    /// Migrate records that match the old requirement (typically from a
    /// configuration file) to the new requirement and resolved version.
    /// Records are migrated across all operating systems and architectures,
    /// as a change in configuration applies to every machine.
    ///
    /// If the resolved version of a migrated record changes, remove the
    /// checksum and source, as they are only valid for the version they
    /// were installed with, and will be re-populated on the next install.
    #[instrument(skip(self))]
    pub fn update_spec_in_lockfile(
        &self,
        old_spec: &UnresolvedVersionSpec,
        new_spec: &UnresolvedVersionSpec,
        new_version: &VersionSpec,
    ) -> Result<(), ProtoLockError> {
        if self.tool.metadata.lock_options.no_record || old_spec == new_spec {
            return Ok(());
        }

        let Some(mut lock) = self.load_lock_mut()? else {
            return Ok(());
        };

        let Some(records) = lock.tools.get_mut(self.tool.get_id()) else {
            return Ok(());
        };

        let backend = self.tool.context.backend.as_ref();
        let mut kept = vec![];
        let mut migrated = vec![];

        for record in std::mem::take(records) {
            if record.backend.as_ref() == backend && record.spec.as_ref() == Some(old_spec) {
                migrated.push(record);
            } else {
                kept.push(record);
            }
        }

        if migrated.is_empty() {
            *records = kept;

            return Ok(());
        }

        debug!(
            tool = self.tool.context.as_str(),
            old_spec = old_spec.to_string(),
            new_spec = new_spec.to_string(),
            "Updating spec for records in lockfile",
        );

        for mut record in migrated {
            // The checksum and source are only valid for the version they
            // were installed with, so remove them when the version changes
            if record.version.as_ref() != Some(new_version) {
                record.checksum = None;
                record.source = None;
            }

            record.spec = Some(new_spec.to_owned());
            record.version = Some(new_version.to_owned());

            // If a record already exists for the new spec, for example from
            // an ad-hoc install, keep the existing record instead, as it may
            // contain a checksum from a real install
            if !kept
                .iter()
                .any(|existing| existing.is_match(&record, &self.tool.metadata.lock_options))
            {
                kept.push(record);
            }
        }

        *records = kept;

        lock.sort_records();
        lock.save()?;

        Ok(())
    }

    /// Remove records that match the requirement (typically from a
    /// configuration file) from the lockfile. Records are removed across
    /// all operating systems and architectures, as a change in
    /// configuration applies to every machine.
    #[instrument(skip(self))]
    pub fn remove_spec_from_lockfile(
        &self,
        spec: &UnresolvedVersionSpec,
    ) -> Result<(), ProtoLockError> {
        let Some(mut lock) = self.load_lock_mut()? else {
            return Ok(());
        };

        let Some(records) = lock.tools.get_mut(self.tool.get_id()) else {
            return Ok(());
        };

        let backend = self.tool.context.backend.as_ref();
        let count = records.len();

        records.retain(|record| {
            !(record.backend.as_ref() == backend && record.spec.as_ref() == Some(spec))
        });

        // Nothing was removed, so avoid saving
        if records.len() == count {
            return Ok(());
        }

        debug!(
            tool = self.tool.context.as_str(),
            spec = spec.to_string(),
            "Removing records for spec from lockfile",
        );

        if records.is_empty() {
            lock.tools.remove(self.tool.get_id());
        }

        lock.sort_records();
        lock.save()?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn resolve_locked_record(
        &self,
        spec: &ToolSpec,
    ) -> Result<Option<LockRecord>, ProtoLockError> {
        let proto = &self.tool.proto;

        let Some(lock) = self.load_lock()? else {
            return Ok(None);
        };

        if let Some(records) = lock.tools.get(self.tool.get_id()) {
            for record in records {
                let matched = record.is_match_with(
                    self.tool.context.backend.as_ref(),
                    Some(&spec.req),
                    Some(&proto.os),
                    Some(&proto.arch),
                    &self.tool.metadata.lock_options,
                );

                if matched && record.version.is_some() {
                    return Ok(Some(record.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Verify the installation is legitimate by comparing it to the internal lockfile record.
    #[instrument(skip(self))]
    pub fn verify_locked_record(
        &self,
        spec: &ToolSpec,
        install_record: &LockRecord,
    ) -> Result<(), ProtoLockError> {
        // Extract the lock record from the lockfile first,
        // otherwise try the internal manifest
        let Some(locked_record) = self.get_resolved_locked_record(spec) else {
            return Ok(());
        };

        // If we have different backends, then the installation strategy
        // and content/files which are hashed may differ, so avoid verify
        if install_record.backend != locked_record.backend {
            return Ok(());
        }

        let make_error = |actual: String, expected: String| match &install_record.source {
            Some(source) => ProtoLockError::MismatchedChecksumWithSource {
                checksum: actual,
                lockfile_checksum: expected,
                source_url: source.to_owned(),
            },
            None => ProtoLockError::MismatchedChecksum {
                checksum: actual,
                lockfile_checksum: expected,
            },
        };

        match (&install_record.checksum, &locked_record.checksum) {
            (Some(ir), Some(lr)) => {
                debug!(
                    tool = self.tool.context.as_str(),
                    checksum = ir.to_string(),
                    locked_checksum = lr.to_string(),
                    "Verifying checksum against lockfile",
                );

                if ir != lr {
                    return Err(make_error(ir.to_string(), lr.to_string()));
                }
            }
            // Only the lockfile has a checksum, so compare the sources.
            // If the sources are the same, something wrong is happening,
            // but if they are different, then it may be a different install
            // strategy, so let it happen
            (None, Some(lr)) if install_record.source == locked_record.source => {
                return Err(make_error("(missing)".into(), lr.to_string()));
            }
            _ => {}
        };

        if let Some(l_os) = install_record.os
            && let Some(r_os) = locked_record.os
            && l_os != r_os
        {
            return Err(ProtoLockError::MismatchedOs {
                os: l_os.to_string(),
                lockfile_os: r_os.to_string(),
            });
        }

        if let Some(l_arch) = install_record.arch
            && let Some(r_arch) = locked_record.arch
            && l_arch != r_arch
        {
            return Err(ProtoLockError::MismatchedArch {
                arch: l_arch.to_string(),
                lockfile_arch: r_arch.to_string(),
            });
        }

        Ok(())
    }
}
