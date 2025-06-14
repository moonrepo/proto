pub use super::lock_error::ProtoLockError;
use crate::lockfile::LockRecord;
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use tracing::{debug, instrument};

impl Tool {
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
    pub fn verify_locked_record(&self, record: &LockRecord) -> Result<(), ProtoLockError> {
        let Some(version) = &self.version else {
            return Ok(());
        };

        // Extract the lock record from the lockfile first,
        // otherwise try the internal manifest
        let Some(locked) = self
            .version_locked
            .as_ref()
            .or_else(|| self.inventory.get_locked_record(version))
        else {
            return Ok(());
        };

        // If we have different backends, then the installation strategy
        // and content/files which are hashed may differ, so avoid verify
        // if record.backend != locked.backend {
        //     return Ok(());
        // }

        let make_error = |actual: String, expected: String| match &record.source {
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

        match (&record.checksum, &locked.checksum) {
            (Some(rc), Some(lc)) => {
                debug!(
                    tool = self.id.as_str(),
                    checksum = rc.to_string(),
                    "Verifying checksum against lockfile",
                );

                if rc != lc {
                    return Err(make_error(rc.to_string(), lc.to_string()));
                }
            }
            // Only the lockfile has a checksum, so compare the sources.
            // If the sources are the same, something wrong is happening,
            // but if they are different, then it may be a different install
            // strategy, so let it happen
            (None, Some(lc)) => {
                if record.source == locked.source {
                    return Err(make_error("(missing)".into(), lc.to_string()));
                }
            }
            _ => {}
        };

        Ok(())
    }
}
