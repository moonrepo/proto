use crate::SchemaPlugin;
use proto_core::{async_trait, Installable, ProtoError, Resolvable};
use std::path::PathBuf;

#[async_trait]
impl Installable<'_> for SchemaPlugin {
    fn get_archive_prefix(&self) -> Result<Option<String>, ProtoError> {
        if let Some(prefix) = &self.get_platform()?.archive_prefix {
            return Ok(Some(self.interpolate_tokens(prefix)));
        }

        Ok(None)
    }

    fn get_install_dir(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.base_dir.join(self.get_resolved_version()))
    }

    fn should_unpack(&self) -> bool {
        self.schema.install.unpack
    }
}
