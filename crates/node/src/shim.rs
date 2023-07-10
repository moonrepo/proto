use crate::NodeLanguage;
use proto_core::{async_trait, create_global_shim, Describable, ProtoError, ShimContext, Shimable};

#[async_trait]
impl Shimable<'_> for NodeLanguage {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        // node
        create_global_shim(ShimContext::new_global(self.get_id()), find_only)?;

        // npx
        create_global_shim(
            ShimContext::new_global_alt(
                self.get_id(),
                "npx",
                if cfg!(windows) { "npx.cmd" } else { "bin/npx" },
            ),
            find_only,
        )?;

        Ok(())
    }
}
