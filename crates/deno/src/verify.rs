use crate::DenoLanguage;
use proto_core::{async_trait, ProtoError, Verifiable};
use std::path::{Path, PathBuf};

#[async_trait]
impl Verifiable<'_> for DenoLanguage {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.temp_dir.join("deno-checksum"))
    }

    fn get_checksum_url(&self) -> Result<String, ProtoError> {
        Ok("".into())
    }

    async fn verify_checksum(
        &self,
        _checksum_file: &Path,
        _download_file: &Path,
    ) -> Result<bool, ProtoError> {
        // Deno doesn't have checksums!
        Ok(true)
    }
}
