use crate::GoLanguage;
use proto_core::{async_trait, create_global_shim, Describable, ProtoError, ShimContext, Shimable};

#[async_trait]
impl Shimable<'_> for GoLanguage {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        create_global_shim(ShimContext::new_global(self.get_id()), find_only)?;

        Ok(())
    }
}
