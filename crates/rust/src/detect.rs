use crate::RustLanguage;
use proto_core::{async_trait, Detector, ProtoError};
use std::path::Path;

#[async_trait]
impl Detector<'_> for RustLanguage {
    async fn detect_version_from(&self, _working_dir: &Path) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }
}
