pub use super::link_error::ProtoLinkError;
use crate::flow::locate::Locator;
use crate::layout::{BinManager, Shim, ShimRegistry, ShimsMap};
use crate::tool::Tool;
use crate::tool_manifest::ToolManifest;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::*;
use proto_shim::*;
use serde::Serialize;
use starbase_utils::{fs, path};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::{debug, instrument, warn};

#[derive(Debug, Default, Serialize)]
pub struct LinkerResponse {
    pub bins: Vec<PathBuf>,
    pub shims: Vec<PathBuf>,
}

/// Link binaries and shims for an installed tool.
pub struct Linker<'tool> {
    tool: &'tool Tool,
    spec: &'tool ToolSpec,
    manifest: Option<&'tool ToolManifest>,
}

impl<'tool> Linker<'tool> {
    pub fn new(tool: &'tool Tool, spec: &'tool ToolSpec) -> Self {
        Self {
            tool,
            spec,
            manifest: None,
        }
    }

    pub fn set_manifest(&mut self, manifest: &'tool ToolManifest) -> &mut Self {
        self.manifest = Some(manifest);
        self
    }

    pub fn unset_manifest(&mut self) -> &mut Self {
        self.manifest = None;
        self
    }

    /// Link both binaries and shims.
    pub async fn link(&self, force: bool) -> Result<LinkerResponse, ProtoLinkError> {
        Ok(LinkerResponse {
            bins: self.link_bins(force).await?,
            shims: self.link_shims(force).await?,
        })
    }

    /// Create shim files for the current tool if they are missing or out of date.
    /// If find only is enabled, will only check if they exist, and not create.
    #[instrument(skip(self))]
    pub async fn link_shims(&self, force: bool) -> Result<Vec<PathBuf>, ProtoLinkError> {
        let shims = Locator::new(self.tool, self.spec).locate_shims().await?;

        if shims.is_empty() {
            return Ok(vec![]);
        }

        let is_outdated = self.tool.inventory.manifest.shim_version != SHIM_VERSION;
        let force_create = force || is_outdated;
        let find_only = !force_create;

        if force_create {
            debug!(
                tool = self.tool.context.as_str(),
                shims_dir = ?self.tool.proto.store.shims_dir,
                shim_version = SHIM_VERSION,
                "Creating shims as they either do not exist, or are outdated"
            );
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

            if !shim.config.primary || shim.name != self.tool.context.id.as_str() {
                shim_entry.parent = Some(self.tool.context.to_string());

                // Only use --alt when the secondary executable exists
                if shim.config.exe_path.is_some() {
                    shim_entry.alt_bin = Some(true);
                }
            }

            // Create the shim file by copying the source executable
            if force_create || find_only && !shim.path.exists() {
                to_create.push(shim.path);
            }

            // Update the registry
            registry.insert(shim.name.clone(), shim_entry);
        }

        // Only create shims if necessary
        if !to_create.is_empty() {
            let store = &self.tool.proto.store;

            fs::create_dir_all(&store.shims_dir)?;

            // Lock for our tests because of race conditions
            #[cfg(debug_assertions)]
            let _lock = fs::lock_directory(&store.shims_dir)?;

            for shim_path in &to_create {
                store.create_shim(shim_path)?;

                debug!(
                    tool = self.tool.context.as_str(),
                    shim = ?shim_path,
                    shim_version = SHIM_VERSION,
                    "Creating shim"
                );
            }

            ShimRegistry::update(&store.shims_dir, registry)?;

            let mut manifest = self.tool.inventory.manifest.clone();
            manifest.shim_version = SHIM_VERSION;
            manifest.save()?;
        }

        Ok(to_create)
    }

    /// Symlink all primary and secondary binaries for the current tool.
    #[instrument(skip(self))]
    pub async fn link_bins(&self, force: bool) -> Result<Vec<PathBuf>, ProtoLinkError> {
        let bins = Locator::new(self.tool, self.spec)
            .locate_bins_with_manager(
                BinManager::from_manifest(self.manifest.unwrap_or(&self.tool.inventory.manifest)),
                if force {
                    None
                } else {
                    self.spec.version.as_ref()
                },
            )
            .await?;

        if bins.is_empty() {
            return Ok(vec![]);
        }

        if force {
            debug!(
                tool = self.tool.context.as_str(),
                bins_dir = ?self.tool.proto.store.bin_dir,
                "Creating symlinks to the original tool executables"
            );
        }

        let mut to_create = vec![];

        for bin in bins {
            let Some(bin_version) = bin.version else {
                continue;
            };

            // Create a new product since we need to change the version for each bin
            let tool_dir = self.tool.inventory.get_product_dir(&bin_version);

            let input_path = tool_dir.join(path::normalize_separators(
                bin.config
                    .exe_link_path
                    .as_ref()
                    .or(bin.config.exe_path.as_ref())
                    .unwrap(),
            ));

            let output_path = bin.path;

            if !input_path.exists() {
                warn!(
                    tool = self.tool.context.as_str(),
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
        let mut bins = vec![];

        if !to_create.is_empty() {
            let store = &self.tool.proto.store;

            fs::create_dir_all(&store.bin_dir)?;

            // Lock for our tests because of race conditions
            #[cfg(debug_assertions)]
            let _lock = fs::lock_directory(&store.bin_dir)?;

            for (input_path, output_path) in to_create {
                debug!(
                    tool = self.tool.context.as_str(),
                    source = ?input_path,
                    target = ?output_path,
                    "Creating binary symlink"
                );

                store.unlink_bin(&output_path)?;
                store.link_bin(&output_path, &input_path)?;

                bins.push(output_path);
            }
        }

        Ok(bins)
    }
}
