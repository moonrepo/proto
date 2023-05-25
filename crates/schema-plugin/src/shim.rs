use crate::SchemaPlugin;
use proto_core::{
    async_trait, Describable, Executable, Installable, ProtoError, Resolvable, ShimBuilder,
    Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for SchemaPlugin {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        let mut shimmer = ShimBuilder::new(self.get_id(), self.get_bin_path()?)?;
        let schema = &self.schema.shim;

        shimmer
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version());

        if let Some(parent_bin) = &schema.parent_bin {
            shimmer.parent(parent_bin);
        }

        if schema.global {
            shimmer.create_global_shim()?;
        }

        if schema.local {
            self.shim_path = Some(shimmer.create_tool_shim(find_only)?);
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
