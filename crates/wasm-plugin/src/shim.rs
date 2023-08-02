use crate::WasmPlugin;
use proto_core::{
    async_trait, create_global_shim, create_global_shim_with_name, create_local_shim, Describable,
    Installable, ProtoError, ShimContext, Shimable,
};
use proto_pdk_api::{CreateShimsInput, CreateShimsOutput};
use std::path::Path;

#[async_trait]
impl Shimable<'_> for WasmPlugin {
    async fn create_shims(&mut self, find_only: bool) -> Result<(), ProtoError> {
        let install_dir = self.get_install_dir()?;
        let mut created_primary = false;

        if self.container.has_func("create_shims") {
            let shim_configs: CreateShimsOutput = self
                .container
                .cache_func_with(
                    "create_shims",
                    CreateShimsInput {
                        env: self.get_environment()?,
                    },
                )
                .map_err(|e| ProtoError::Message(e.to_string()))?;

            created_primary = shim_configs.no_primary_global;

            if let Some(primary_config) = &shim_configs.primary {
                let mut context = ShimContext::new_global(self.get_id());
                context.parent_bin = primary_config.parent_bin.as_deref();
                context.before_args = primary_config.before_args.as_deref();
                context.after_args = primary_config.after_args.as_deref();
                context.globals_bin_dir = Some(&self.proto.bin_dir);

                if !shim_configs.no_primary_global {
                    create_global_shim(context, find_only)?;
                }

                created_primary = true;
            }

            for (name, config) in &shim_configs.global_shims {
                let mut context = if let Some(alt_bin) = &config.bin_path {
                    ShimContext::new_global_alt(self.get_id(), name, alt_bin)
                } else {
                    ShimContext::new_global(self.get_id())
                };

                context.before_args = config.before_args.as_deref();
                context.after_args = config.after_args.as_deref();
                context.globals_bin_dir = Some(&self.proto.bin_dir);

                if config.bin_path.is_some() {
                    create_global_shim(context, find_only)?;
                } else {
                    create_global_shim_with_name(context, name, find_only)?;
                }
            }

            for (name, config) in &shim_configs.local_shims {
                let bin_path = install_dir.join(config.bin_path.as_ref().unwrap_or(name));

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
            let mut context = ShimContext::new_global(self.get_id());
            context.globals_bin_dir = Some(&self.proto.bin_dir);

            create_global_shim(context, find_only)?;
        }

        Ok(())
    }

    fn get_shim_path(&self) -> Option<&Path> {
        self.shim_path.as_deref()
    }
}
