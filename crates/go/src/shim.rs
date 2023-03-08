use crate::GoLanguage;
use proto_core::{
    async_trait, Describable, Executable, Installable, ProtoError, Resolvable, ShimBuilder,
    Shimable,
};

#[async_trait]
impl Shimable<'_> for GoLanguage {
    async fn create_shims(&mut self) -> Result<(), ProtoError> {
        let mut shimmer = ShimBuilder::new(self.get_bin_name(), self.get_bin_path()?);

        shimmer
            .dir(self.get_install_dir()?)
            .version(self.get_resolved_version());

        shimmer.create_global_shim()?;

        // No tool shim

        Ok(())
    }
}
