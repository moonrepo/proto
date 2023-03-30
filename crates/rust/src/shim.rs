use crate::RustLanguage;
use proto_core::{async_trait, ProtoError, Shimable};

#[async_trait]
impl Shimable<'_> for RustLanguage {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        Ok(())
    }
}
