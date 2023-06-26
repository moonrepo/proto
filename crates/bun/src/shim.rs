use crate::BunLanguage;
use proto_core::{
    async_trait, create_global_shim, create_global_shim_with_name, Describable, ProtoError,
    ShimContext, Shimable,
};

#[async_trait]
impl Shimable<'_> for BunLanguage {
    async fn create_shims(&mut self, _find_only: bool) -> Result<(), ProtoError> {
        // bun
        let mut context = ShimContext::new_global(self.get_id());

        create_global_shim(&context)?;

        // bunx
        context.before_args = Some("x");

        create_global_shim_with_name(context, "bunx")?;

        Ok(())
    }
}
