use crate::RustLanguage;
use proto_core::{async_trait, Describable, Executable, ProtoError};
use std::path::Path;

#[async_trait]
impl Executable<'_> for RustLanguage {
    async fn find_bin_path(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }

    fn get_bin_path(&self) -> Result<&Path, ProtoError> {
        Err(ProtoError::MissingTool(self.get_name()))
    }
}
