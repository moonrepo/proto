use crate::helpers::now;
use starbase_utils::fs;
use std::path::PathBuf;
use version_spec::VersionSpec;

#[derive(Clone, Default, Debug)]
pub struct Product {
    pub dir: PathBuf,
    pub version: VersionSpec,
}

impl Product {
    pub fn load_used_at(&self) -> miette::Result<Option<u128>> {
        let file = self.dir.join(".last-used");

        if file.exists() {
            if let Ok(contents) = fs::read_file(file) {
                if let Ok(value) = contents.parse::<u128>() {
                    return Ok(Some(value));
                }
            }
        }

        Ok(None)
    }

    pub fn track_used_at(&self) -> miette::Result<()> {
        fs::write_file(self.dir.join(".last-used"), now().to_string())?;

        Ok(())
    }
}
