pub use super::link_error::ProtoLinkError;
use crate::flow::locate::Locator;
use crate::layout::{BinManager, Shim, ShimRegistry};
use crate::tool::Tool;
use crate::tool_spec::ToolSpec;
use proto_pdk_api::*;
use proto_shim::*;
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::{fs, path};
use std::path::PathBuf;
use tracing::{debug, instrument, warn};

#[derive(Clone, Debug, Default, Serialize)]
pub struct LinkerResponse {
    pub bins: Vec<PathBuf>,
    pub shims: Vec<PathBuf>,
}

/// Link binaries and shims for an installed tool.
pub struct Linker<'tool> {
    tool: &'tool Tool,
    spec: &'tool ToolSpec,
    shim_registry: ShimRegistry,
}

impl<'tool> Linker<'tool> {
    pub fn new(tool: &'tool Tool, spec: &'tool ToolSpec) -> Result<Self, ProtoLinkError> {
        Ok(Self {
            shim_registry: tool.proto.store.load_shims_registry()?,
            tool,
            spec,
        })
    }

    #[instrument]
    pub async fn link(
        tool: &'tool Tool,
        spec: &'tool ToolSpec,
        force: bool,
    ) -> Result<LinkerResponse, ProtoLinkError> {
        Self::new(tool, spec)?.link_all(force).await
    }

    /// Link both binaries and shims.
    #[instrument(skip(self))]
    pub async fn link_all(&mut self, force: bool) -> Result<LinkerResponse, ProtoLinkError> {
        // Shims are linked first so they populate the executable ownership
        // registry, which bin linking consults to avoid clobbering the
        // binaries of another tool that already owns the same name.
        let shims = self.link_shims(force).await?;
        let bins = self.link_bins(force).await?;

        Ok(LinkerResponse { bins, shims })
    }

    /// Create shim files for the current tool if they are missing or out of date.
    /// If find only is enabled, will only check if they exist, and not create.
    #[instrument(skip(self))]
    pub async fn link_shims(&mut self, force: bool) -> Result<Vec<PathBuf>, ProtoLinkError> {
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
                shim_entry.context = Some(self.tool.context.clone());

                // Only use --alt when the secondary executable exists
                if shim.config.exe_path.is_some() {
                    shim_entry.alt_exe = Some(true);
                }
            }

            // Tools that require a backend must always set a context
            if self.tool.context.backend.is_some() && shim_entry.context.is_none() {
                shim_entry.context = Some(self.tool.context.clone());
            }

            // Create the shim file by copying the source executable
            if force_create || find_only && !shim.path.exists() {
                to_create.push(shim.path);
            }

            // Update the registry
            self.shim_registry.update(shim.name, shim_entry)?;
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

            let mut manifest = self.tool.inventory.manifest.clone();
            manifest.shim_version = SHIM_VERSION;
            manifest.save()?;
        }

        self.shim_registry.save()?;

        Ok(to_create)
    }

    /// Symlink all primary and secondary binaries for the current tool.
    #[instrument(skip(self))]
    pub async fn link_bins(&self, force: bool) -> Result<Vec<PathBuf>, ProtoLinkError> {
        let bins = Locator::new(self.tool, self.spec)
            .locate_bins(if force {
                None
            } else {
                self.spec.version.as_ref()
            })
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

        // Ownership of an executable name is tracked in the shims registry
        // (populated by `link_shims`, which runs first). Bins are consulted
        // against it so a tool can't clobber a binary owned by another tool.
        let mut to_create = vec![];

        for bin in bins {
            let Some(bin_version) = bin.version else {
                continue;
            };

            // Skip bins for executables owned by a different tool. The registry
            // is keyed by the bare executable name, so this naturally targets
            // the primary `*`-bucket bins; versioned bins are uniquely named.
            if let Some(entry) = self.shim_registry.get(&bin.name) {
                let owned_by_this = match &entry.context {
                    Some(owner) => owner == &self.tool.context,
                    None => self.tool.context.id.as_str() == bin.name,
                };

                if !owned_by_this {
                    let owner = entry
                        .context
                        .as_ref()
                        .map(|ctx| ctx.as_str())
                        .unwrap_or(bin.name.as_str());

                    warn!(
                        tool = self.tool.context.as_str(),
                        exe = bin.name.as_str(),
                        owner = owner,
                        "Skipping linking binary {}, already provided by {}",
                        color::file(&bin.name),
                        color::id(owner)
                    );

                    continue;
                }
            }

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

    /// Remove all binaries for the tool across every installed version.
    #[instrument(skip(self))]
    pub async fn unlink_bins(&self) -> Result<(), ProtoLinkError> {
        let bin_manager = BinManager::from_manifest(&self.tool.inventory.manifest);

        for bin in Locator::new(self.tool, self.spec)
            .locate_bins_with_manager(&bin_manager, None)
            .await?
        {
            self.tool.proto.store.unlink_bin(&bin.path)?;
        }

        Ok(())
    }

    /// Reconcile binaries after a version has been removed from the tool. Bins
    /// whose bucket no longer maps to any version are removed, while bins whose
    /// bucket is reassigned to a remaining version are re-pointed to it. Must be
    /// called *before* the version is removed from the manifest, as the buckets
    /// are recomputed from it.
    #[instrument(skip(self))]
    pub async fn unlink_bins_by_version(
        &self,
        version: &VersionSpec,
    ) -> Result<(), ProtoLinkError> {
        let store = &self.tool.proto.store;
        let locator = Locator::new(self.tool, self.spec);
        let mut bin_manager = BinManager::from_manifest(&self.tool.inventory.manifest);

        // Snapshot the affected bins before removal
        let old_bins = locator
            .locate_bins_with_manager(&bin_manager, Some(version))
            .await?;

        // If this version didn't occupy any bucket, there are no bins to change
        if !bin_manager.remove_version(version) {
            return Ok(());
        }

        let new_bins = locator
            .locate_bins_with_manager(&bin_manager, Some(version))
            .await?;
        let new_bins_by_path = new_bins
            .iter()
            .map(|bin| (&bin.path, bin))
            .collect::<FxHashMap<_, _>>();

        for old_bin in &old_bins {
            match new_bins_by_path.get(&old_bin.path) {
                // Bucket no longer exists, remove the orphaned bin
                None => {
                    store.unlink_bin(&old_bin.path)?;
                }
                // Bucket reassigned to another version, re-point it
                Some(new_bin) if new_bin.version != old_bin.version => {
                    if let (Some(new_version), Some(exe_path)) = (
                        new_bin.version.as_ref(),
                        new_bin
                            .config
                            .exe_link_path
                            .as_ref()
                            .or(new_bin.config.exe_path.as_ref()),
                    ) {
                        let src_path = self
                            .tool
                            .inventory
                            .get_product_dir(new_version)
                            .join(path::normalize_separators(exe_path));

                        if src_path.exists() {
                            store.unlink_bin(&new_bin.path)?;
                            store.link_bin(&new_bin.path, &src_path)?;
                        }
                    }
                }
                // Bucket unchanged, leave the bin as-is
                Some(_) => {}
            }
        }

        Ok(())
    }

    /// Remove all shims for the tool.
    #[instrument(skip(self))]
    pub async fn unlink_shims(&self) -> Result<(), ProtoLinkError> {
        for shim in Locator::new(self.tool, self.spec).locate_shims().await? {
            self.tool.proto.store.remove_shim(&shim.path)?;
        }

        Ok(())
    }
}
