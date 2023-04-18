use crate::SchemaPlugin;
use proto_core::{async_trait, load_version_file, Detector, ProtoError};
use std::path::Path;

#[async_trait]
impl Detector<'_> for SchemaPlugin {
    async fn detect_version_from(&self, working_dir: &Path) -> Result<Option<String>, ProtoError> {
        if let Some(version_files) = &self.schema.detect.version_files {
            for file in version_files {
                let file_path = working_dir.join(file);

                if file_path.exists() {
                    return Ok(Some(load_version_file(&file_path)?));
                }
            }
        }

        Ok(None)
    }
}
