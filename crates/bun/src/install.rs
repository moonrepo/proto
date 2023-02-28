use crate::{download::get_archive_file_path, BunLanguage};
use proto_core::{async_trait, Installable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Installable<'_> for BunLanguage {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        Ok(Some(get_archive_file_path()?))
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.base_dir.join(self.get_resolved_version()))
    }
}
