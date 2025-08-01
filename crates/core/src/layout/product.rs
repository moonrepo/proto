use super::layout_error::ProtoLayoutError;
use crate::helpers::now;
use starbase_utils::fs;
use std::path::PathBuf;
use tracing::instrument;
use version_spec::VersionSpec;

#[derive(Clone, Default, Debug)]
pub struct Product {
    pub dir: PathBuf,
    pub version: VersionSpec,
}

impl Product {
    #[instrument(skip(self))]
    pub fn load_used_at(&self) -> Result<Option<u128>, ProtoLayoutError> {
        let file = self.dir.join(".last-used");

        if file.exists()
            && let Ok(contents) = fs::read_file(file)
            && let Ok(value) = contents.parse::<u128>()
        {
            return Ok(Some(value));
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub fn track_used_at(&self) -> Result<(), ProtoLayoutError> {
        // Directory may have been deleted by auto-clean,
        // so avoid writing this file to an empty directory!
        if self.dir.exists() && self.dir.is_dir() {
            fs::write_file(self.dir.join(".last-used"), now().to_string())?;
        }

        Ok(())
    }
}
