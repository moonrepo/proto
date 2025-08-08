pub use super::link_error::ProtoLinkError;
use crate::helpers::normalize_path_separators;
use crate::layout::{Shim, ShimRegistry, ShimsMap};
use crate::tool::Tool;
use proto_pdk_api::*;
use proto_shim::*;
use starbase_utils::fs;
use std::collections::BTreeMap;
use tracing::{debug, instrument, warn};

impl Tool {
    /// Create shim files for the current tool if they are missing or out of date.
    /// If find only is enabled, will only check if they exist, and not create.
    #[instrument(skip(self))]
    pub async fn generate_shims(&mut self, force: bool) -> Result<(), ProtoLinkError> {
        let shims = self.resolve_shim_locations().await?;

        if shims.is_empty() {
            return Ok(());
        }

        let is_outdated = self.inventory.manifest.shim_version != SHIM_VERSION;
        let force_create = force || is_outdated;
        let find_only = !force_create;

        if force_create {
            debug!(
                tool = self.context.as_str(),
                shims_dir = ?self.proto.store.shims_dir,
                shim_version = SHIM_VERSION,
                "Creating shims as they either do not exist, or are outdated"
            );

            self.inventory.manifest.shim_version = SHIM_VERSION;
            self.inventory.manifest.save()?;
        }

        let mut registry: ShimsMap = BTreeMap::default();
        let mut to_create = vec![];

        for shim in shims {
            let mut shim_entry = Shim::default();

            // Handle before and after args
            if let Some(before_args) = shim.config.shim_before_args {
                shim_entry.before_args = match before_args {
                    StringOrVec::String(value) => shell_words::split(&value).map_err(|error| {
                        ProtoLinkError::FailedArgsParse {
                            args: value,
                            error: Box::new(error),
                        }
                    })?,
                    StringOrVec::Vec(value) => value,
                };
            }

            if let Some(after_args) = shim.config.shim_after_args {
                shim_entry.after_args = match after_args {
                    StringOrVec::String(value) => shell_words::split(&value).map_err(|error| {
                        ProtoLinkError::FailedArgsParse {
                            args: value,
                            error: Box::new(error),
                        }
                    })?,
                    StringOrVec::Vec(value) => value,
                };
            }

            if let Some(env_vars) = shim.config.shim_env_vars {
                shim_entry.env_vars.extend(env_vars);
            }

            if !shim.config.primary {
                shim_entry.parent = Some(self.get_id().to_string());

                // Only use --alt when the secondary executable exists
                if shim.config.exe_path.is_some() {
                    shim_entry.alt_bin = Some(true);
                }
            }

            // Create the shim file by copying the source bin
            if force_create || find_only && !shim.path.exists() {
                to_create.push(shim.path);
            }

            // Update the registry
            registry.insert(shim.name.clone(), shim_entry);
        }

        // Only create shims if necessary
        if !to_create.is_empty() {
            fs::create_dir_all(&self.proto.store.shims_dir)?;

            // Lock for our tests because of race conditions
            #[cfg(debug_assertions)]
            let _lock = fs::lock_directory(&self.proto.store.shims_dir)?;

            for shim_path in to_create {
                self.proto.store.create_shim(&shim_path)?;

                debug!(
                    tool = self.context.as_str(),
                    shim = ?shim_path,
                    shim_version = SHIM_VERSION,
                    "Creating shim"
                );
            }

            ShimRegistry::update(&self.proto.store.shims_dir, registry)?;
        }

        Ok(())
    }

    /// Symlink all primary and secondary binaries for the current tool.
    #[instrument(skip(self))]
    pub async fn symlink_bins(&mut self, force: bool) -> Result<(), ProtoLinkError> {
        let bins = self.resolve_bin_locations(force).await?;

        if bins.is_empty() {
            return Ok(());
        }

        if force {
            debug!(
                tool = self.context.as_str(),
                bins_dir = ?self.proto.store.bin_dir,
                "Creating symlinks to the original tool executables"
            );
        }

        let mut to_create = vec![];

        for bin in bins {
            let Some(bin_version) = bin.version else {
                continue;
            };

            // Create a new product since we need to change the version for each bin
            let tool_dir = self.inventory.create_product(&bin_version).dir;

            let input_path = tool_dir.join(normalize_path_separators(
                bin.config
                    .exe_link_path
                    .as_ref()
                    .or(bin.config.exe_path.as_ref())
                    .unwrap(),
            ));

            let output_path = bin.path;

            if !input_path.exists() {
                warn!(
                    tool = self.context.as_str(),
                    source = ?input_path,
                    target = ?output_path,
                    "Unable to symlink binary, source file does not exist"
                );

                continue;
            }

            if !force && output_path.exists() {
                continue;
            }

            to_create.push((input_path, output_path));
        }

        // Only create bins if necessary
        if !to_create.is_empty() {
            fs::create_dir_all(&self.proto.store.bin_dir)?;

            // Lock for our tests because of race conditions
            #[cfg(debug_assertions)]
            let _lock = fs::lock_directory(&self.proto.store.bin_dir)?;

            for (input_path, output_path) in to_create {
                debug!(
                    tool = self.context.as_str(),
                    source = ?input_path,
                    target = ?output_path,
                    "Creating binary symlink"
                );

                self.proto.store.unlink_bin(&output_path)?;
                self.proto.store.link_bin(&output_path, &input_path)?;
            }
        }

        Ok(())
    }
}
