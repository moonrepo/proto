use crate::SchemaPlugin;
use proto_core::{
    async_trait, create_global_shim, create_local_shim, Describable, Executable, Installable,
    ProtoError, Resolvable, ShimContext, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for SchemaPlugin {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        let schema = &self.schema.shim;

        if schema.global {
            create_global_shim(ShimContext::new_global(self.get_id()))?;
        }

        if schema.local {
            let install_dir = self.get_install_dir()?;

            let mut context = ShimContext::new_local(self.get_id(), self.get_bin_path()?);
            context.parent_bin = schema.parent_bin.as_deref();
            context.tool_dir = Some(&install_dir);
            context.tool_version = Some(self.get_resolved_version());

            create_local_shim(context, find_only)?;
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
