pub use super::lock_error::ProtoLockError;
use crate::lockfile::LockRecord;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use tracing::{debug, instrument};

// [ ] install many
//      [x] resolve version from lockfile
//      [x] validate lock record
//      [ ] create lockfile if it does not exist
//      [ ] error if spec/req is not found in lockfile
// [ ] install one
//      [x] resolve version from lockfile
//      [x] validate lock record
// [x] install one version
//      [x] don't resolve version from lockfile
//      [x] validate lock record
// [ ] outdated
//      [ ] add locked label to table
//      [ ] integrate with --update
// [x] run
//      [x] resolve version from lockfile
// [ ] status
//      [ ] add locked label to table

impl Tool {
    pub fn get_locked_record(&self) -> Option<&LockRecord> {
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

    pub fn get_locked_records(&self) -> Result<Option<Vec<&LockRecord>>, ProtoLockError> {
        Ok(self
            .proto
            .load_lock()?
            .and_then(|lock| lock.tools.get(&self.id))
            .map(|records| records.iter().collect()))
    }

    #[instrument(skip(self))]
    pub fn resolve_locked_record(
        &self,
        spec: &ToolSpec,
    ) -> Result<Option<LockRecord>, ProtoLockError> {
        if let Some(records) = self.get_locked_records()? {
            for record in records {
                if spec.backend == record.backend
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
        let Some(locked_record) = self.get_locked_record() else {
            return Ok(());
        };

        // If we have different backends, then the installation strategy
        // and content/files which are hashed may differ, so avoid verify
        // if record.backend != locked.backend {
        //     return Ok(());
        // }

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
                    tool = self.id.as_str(),
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
