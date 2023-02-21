use crate::DenoLanguage;
use proto_core::{async_trait, load_version_file, Detector, ProtoError};
use std::path::Path;

#[async_trait]
impl Detector<'_> for DenoLanguage {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        let dvmrc = working_dir.join(".dvmrc");

        if dvmrc.exists() {
            return Ok(Some(load_version_file(&dvmrc)?));
        }

        Ok(None)
    }
}
