use crate::RustLanguage;
use proto_core::{async_trait, ProtoError, Shimable};

#[async_trait]
impl Shimable<'_> for RustLanguage {
    // Don't create shims and rely on binaries found in `~/.cargo/bin`.
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }
}
