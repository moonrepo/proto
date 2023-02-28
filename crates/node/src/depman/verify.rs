use crate::depman::NodeDependencyManager;
use proto_core::{async_trait, ProtoError, Resolvable, Verifiable};
use std::path::{Path, PathBuf};

// TODO: implement PGP/ECDSA signature verify
// https://docs.npmjs.com/about-registry-signatures
#[async_trait]
impl Verifiable<'_> for NodeDependencyManager {
    fn get_checksum_path(&self) -> Result<PathBuf, ProtoError> {
        Ok(self
            .temp_dir
            .join(format!("v{}.pub", self.get_resolved_version())))
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
