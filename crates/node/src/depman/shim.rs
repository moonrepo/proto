use crate::depman::NodeDependencyManagerType;
use crate::NodeDependencyManager;
use proto_core::{
    async_trait, create_global_shim, create_local_shim, Executable, Installable, ProtoError,
    Resolvable, ShimContext, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for NodeDependencyManager {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        let install_dir = self.get_install_dir()?;

        // npm
        let mut context = ShimContext::new_local(&self.package_name, self.get_bin_path()?);
        context.parent_bin = Some("node");

        create_global_shim(&context)?;

        context.tool_dir = Some(&install_dir);
        context.tool_version = Some(self.get_resolved_version());

        self.shim_path = Some(create_local_shim(context, find_only)?);

        // node-gyp
        if matches!(self.type_of, NodeDependencyManagerType::Npm) {
            create_global_shim(ShimContext::new_global_alt(
                "npm",
                "node-gyp",
                if cfg!(windows) {
                    "node-gyp-bin/node-gyp.cmd"
                } else {
                    "node-gyp-bin/node-gyp"
                },
            ))?;
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
