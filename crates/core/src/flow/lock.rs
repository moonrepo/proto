pub use super::lock_error::ProtoLockError;
use crate::lockfile::LockRecord;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use tracing::{debug, instrument};
use version_spec::VersionSpec;

// [x] install many
//      [x] resolve version from lockfile
//      [x] validate lock record
//      [x] create lockfile if it does not exist
//      [x] error if spec/req is not found in lockfile
// [x] install one
//      [x] resolve version from lockfile
//      [x] validate lock record
// [x] install one version
//      [x] don't resolve version from lockfile
//      [x] validate lock record
// [x] uninstall
//      [x] remove from lockfile
// [ ] outdated
//      [ ] add locked label to table
//      [ ] integrate with --update
// [x] run
//      [x] resolve version from lockfile
// [ ] status
//      [ ] add locked label to table

impl Tool {
    pub fn insert_record_into_lockfile(&self, record: &LockRecord) -> Result<(), ProtoLockError> {
        let Some(mut lock) = self.proto.load_lock_mut()? else {
            return Ok(());
        };

        let record = record.for_lockfile();
        let records = lock.tools.entry(self.get_id().to_owned()).or_default();

        // Find an existing record with the same spec
        match records
            .iter_mut()
            .find(|existing| existing.backend == record.backend && existing.spec == record.spec)
        {
            Some(existing) => {
                // If the new record has a higher version,
                // we should replace the existing record with it
                if record.version.as_ref().unwrap() >= existing.version.as_ref().unwrap() {
                    *existing = record;
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
        let Some(mut lock) = self.proto.load_lock_mut()? else {
            return Ok(());
        };

        lock.tools.remove(self.get_id());
        lock.save()?;

        Ok(())
    }

    pub fn remove_version_from_lockfile(
        &self,
        version: &VersionSpec,
    ) -> Result<(), ProtoLockError> {
        let Some(mut lock) = self.proto.load_lock_mut()? else {
            return Ok(());
        };

        if let Some(records) = lock.tools.get_mut(self.get_id()) {
            records.retain(|record| {
                !(record.backend == self.context.backend
                    && record.spec.as_ref().is_some_and(|spec| spec == version)
                    && record.version.as_ref().is_some_and(|ver| ver == version))
            });
        }

        if lock
            .tools
            .get(self.get_id())
            .is_none_or(|records| records.is_empty())
        {
            lock.tools.remove(self.get_id());
        }

        lock.sort_records();
        lock.save()?;

        Ok(())
    }

    pub fn get_resolved_locked_record(&self) -> Option<&LockRecord> {
        // From lockfile (after being resolved)
        if let Some(record) = &self.version_locked {
            return Some(record);
        }

        // From manifest
        if let Some(version) = &self.version {
            return self.inventory.get_locked_record(version);
        }

        None
    }

    #[instrument(skip(self))]
    pub fn resolve_locked_record(
        &self,
        spec: &ToolSpec,
    ) -> Result<Option<LockRecord>, ProtoLockError> {
        let Some(lock) = self.proto.load_lock()? else {
            return Ok(None);
        };

        if let Some(records) = lock.tools.get(self.get_id()) {
            for record in records {
                if record.backend == self.context.backend
                    && record.version.is_some()
                    && record
                        .spec
                        .as_ref()
                        .is_some_and(|rec_spec| rec_spec == &spec.req)
                {
                    return Ok(Some(record.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Verify the installation is legitimate by comparing it to the internal lockfile record.
    #[instrument(skip(self))]
    pub fn verify_locked_record(&self, install_record: &LockRecord) -> Result<(), ProtoLockError> {
        // Extract the lock record from the lockfile first,
        // otherwise try the internal manifest
        let Some(locked_record) = self.get_resolved_locked_record() else {
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
                    tool = self.context.as_str(),
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
            (None, Some(lr)) => {
                if install_record.source == locked_record.source {
                    return Err(make_error("(missing)".into(), lr.to_string()));
                }
            }
            _ => {}
        };

        Ok(())
    }
}
