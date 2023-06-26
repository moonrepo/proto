use crate::BunLanguage;
use proto_core::{async_trait, create_global_shim, Describable, ProtoError, ShimContext, Shimable};

#[async_trait]
impl Shimable<'_> for BunLanguage {
    async fn create_shims(&mut self, _find_only: bool) -> Result<(), ProtoError> {
        create_global_shim(ShimContext::new_global(self.get_id()))?;

        Ok(())
    }
}
