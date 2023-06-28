use crate::WasmPlugin;
use proto_core::{
    async_trait, create_global_shim, create_local_shim, Describable, Installable, ProtoError,
    ShimContext, Shimable,
};
use proto_pdk::{ShimParamsInput, ShimParamsOutput};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for WasmPlugin {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        let install_dir = self.get_install_dir()?;
        let mut created_primary = false;

        if self.has_func("register_shims") {
            let shim_configs: ShimParamsOutput = self.cache_func_with(
                "register_shims",
                ShimParamsInput {
                    env: self.get_environment()?,
                },
            )?;

            if let Some(primary_config) = &shim_configs.primary {
                let mut context = ShimContext::new_global(self.get_id());
                context.before_args = primary_config.before_args.as_deref();
                context.after_args = primary_config.after_args.as_deref();

                create_global_shim(context)?;

                created_primary = true;
            }

            for (name, alt_bin) in &shim_configs.global_shims {
                create_global_shim(ShimContext::new_global_alt(self.get_id(), name, alt_bin))?;
            }

            for (name, config) in &shim_configs.local_shims {
                let bin_path = install_dir.join(&config.bin_path);

                let mut context = ShimContext::new_local(name, &bin_path);
                context.parent_bin = config.parent_bin.as_deref();
                context.before_args = config.before_args.as_deref();
                context.after_args = config.after_args.as_deref();
                context.tool_dir = Some(&install_dir);

                let shim_path = create_local_shim(context, find_only)?;

                if name == self.get_id() {
                    self.shim_path = Some(shim_path);
                }
            }
        }

        // We must always create a primary global shim, so if the plugin did not configure one,
        // we will create one automatically using the information we have.
        if !created_primary {
            create_global_shim(ShimContext::new_global(self.get_id()))?;
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
