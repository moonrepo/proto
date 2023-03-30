use crate::RustLanguage;
use proto_core::{async_trait, ProtoError, Verifiable};
use std::path::{Path, PathBuf};

#[async_trait]
impl Verifiable<'_> for RustLanguage {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self.temp_dir.join("checksum"))
    }

    fn get_checksum_url(&self) -> Result<Option<String>, ProtoError> {
        Ok(None)
    }

    async fn verify_checksum(
        &self,
        _checksum_file: &Path,
        _download_file: &Path,
    ) -> Result<bool, ProtoError> {
        Ok(true)
    }
}
