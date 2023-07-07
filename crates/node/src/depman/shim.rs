use crate::depman::NodeDependencyManagerType;
use crate::NodeDependencyManager;
use proto_core::{
    async_trait, create_global_shim, create_global_shim_with_name, create_local_shim, Executable,
    Installable, ProtoError, Resolvable, ShimContext, Shimable,
};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for NodeDependencyManager {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        let install_dir = self.get_install_dir()?;

        // npm, pnpm, yarn
        let mut context = ShimContext::new_local(&self.package_name, self.get_bin_path()?);
        context.parent_bin = Some("node");

        create_global_shim(&context)?;

        match self.type_of {
            // node-gyp
            NodeDependencyManagerType::Npm => {
                create_global_shim(ShimContext::new_global_alt(
                    "npm",
                    "node-gyp",
                    if cfg!(windows) {
                        "bin/node-gyp-bin/node-gyp.cmd"
                    } else {
                        "bin/node-gyp-bin/node-gyp"
                    },
                ))?;
            }

            // pnpx
            NodeDependencyManagerType::Pnpm => {
                let mut context = ShimContext::new_global("pnpm");
                context.before_args = Some("dlx");

                create_global_shim_with_name(&context, "pnpx")?;
            }

            // yarnpkg
            NodeDependencyManagerType::Yarn => {
                create_global_shim_with_name(&context, "yarnpkg")?;
            }
        };

        context.tool_dir = Some(&install_dir);
        context.tool_version = Some(self.get_resolved_version());

        self.shim_path = Some(create_local_shim(&context, find_only)?);

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
