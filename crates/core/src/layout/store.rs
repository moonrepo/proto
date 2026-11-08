use super::inventory::Inventory;
use super::layout_error::ProtoLayoutError;
use crate::id::Id;
use crate::layout::ShimRegistry;
use crate::tool_manifest::ToolManifest;
use once_cell::sync::OnceCell;
use proto_pdk_api::ToolInventoryOptions;
use proto_shim::{create_shim, locate_proto_exe};
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::fs::FsError;
use starbase_utils::{envx, fs, path};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};
use tracing::{debug, instrument, warn};

fn symlink_file(src_path: &Path, link_path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src_path, link_path)
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(src_path, link_path)
    }
}

#[derive(Clone, Default, Serialize)]
pub struct Store {
    pub dir: PathBuf,
    pub backends_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub builders_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub inventory_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub plugins_temp_dir: PathBuf,
    pub shims_dir: PathBuf,
    pub temp_dir: PathBuf,

    #[serde(skip)]
    shim_binary: Arc<OnceCell<PathBuf>>,
}

impl Store {
    #[instrument(name = "create_store")]
    pub fn new(dir: &Path) -> Self {
        let temp_dir = match envx::path_var("PROTO_TEMP_DIR") {
            Some(custom) => {
                debug!(
                    temp_dir = ?custom,
                    "Using custom temp directory from {}",
                    color::symbol("PROTO_TEMP_DIR")
                );

                custom
            }
            None => dir.join("temp"),
        };

        // When the plugins directory is relocated, it may be shared by many
        // concurrent proto processes that each have their own store. The loader
        // coordinates downloads through lock files in its temp directory, so
        // that must travel with the plugins directory, otherwise each process
        // would lock a path that the others never look at.
        let (plugins_dir, plugins_temp_dir) = match envx::path_var("PROTO_PLUGINS_DIR") {
            Some(custom) => {
                debug!(
                    plugins_dir = ?custom,
                    "Using custom plugins directory from {}",
                    color::symbol("PROTO_PLUGINS_DIR")
                );

                let temp = custom.join(".temp");

                (custom, temp)
            }
            None => (dir.join("plugins"), temp_dir.join("plugins")),
        };

        Self {
            dir: dir.to_path_buf(),
            backends_dir: dir.join("backends"),
            bin_dir: dir.join("bin"),
            builders_dir: dir.join("builders"),
            cache_dir: dir.join("cache"),
            inventory_dir: dir.join("tools"),
            plugins_dir,
            plugins_temp_dir,
            shims_dir: dir.join("shims"),
            temp_dir,
            shim_binary: Arc::new(OnceCell::new()),
        }
    }

    pub fn create_inventory(
        &self,
        id: &Id,
        config: &ToolInventoryOptions,
    ) -> Result<Inventory, ProtoLayoutError> {
        let dir = self.inventory_dir.join(path::encode_component(id));

        Ok(Inventory {
            manifest: ToolManifest::load_from(&dir)?,
            dir,
            dir_original: None,
            temp_dir: self.temp_dir.join(path::encode_component(id)),
            config: config.to_owned(),
        })
    }

    pub fn load_uuid(&self) -> Result<String, ProtoLayoutError> {
        let id_path = self.dir.join("id");

        if id_path.exists() {
            return Ok(fs::read_file(id_path)?);
        }

        let id = uuid::Uuid::new_v4().to_string();

        fs::write_file(id_path, &id)?;

        Ok(id)
    }

    #[instrument(skip(self))]
    pub fn load_preferred_profile(&self) -> Result<Option<PathBuf>, ProtoLayoutError> {
        let profile_path = self.dir.join("profile");

        if profile_path.exists() {
            return Ok(Some(fs::read_file(profile_path)?.into()));
        }

        Ok(None)
    }

    #[instrument(skip(self))]
    pub fn locate_shim_binary(&self) -> Result<&Path, ProtoLayoutError> {
        let path = self.shim_binary.get_or_try_init(|| {
            locate_proto_exe("proto-shim").ok_or_else(|| ProtoLayoutError::MissingShimBinary {
                bin_dir: self.bin_dir.clone(),
            })
        })?;

        Ok(path)
    }

    #[instrument(skip(self))]
    pub fn load_shims_registry(&self) -> Result<ShimRegistry, ProtoLayoutError> {
        ShimRegistry::load_from(&self.shims_dir)
    }

    #[instrument(skip(self))]
    pub fn save_preferred_profile(&self, path: &Path) -> Result<(), ProtoLayoutError> {
        fs::write_file(
            self.dir.join("profile"),
            path.as_os_str().as_encoded_bytes(),
        )?;

        Ok(())
    }

    /// Symlink an executable into the bin directory, and return whether it was linked.
    #[instrument(skip(self))]
    pub fn link_bin(&self, bin_path: &Path, src_path: &Path) -> Result<bool, ProtoLayoutError> {
        let result = symlink_file(src_path, bin_path);

        // Symlinks require developer mode or elevated privileges on Windows.
        // If we don't have access, skip the executable, as the tool
        // is still available through its shim.
        if cfg!(windows)
            && let Err(error) = &result
        {
            static WARNED: Once = Once::new();

            // Every executable fails for the same reason, so only warn once
            WARNED.call_once(|| {
                warn!(
                    bin_dir = ?self.bin_dir,
                    "Unable to symlink binaries, skipping them: {error}"
                );
            });

            debug!(
                source = ?src_path,
                target = ?bin_path,
                "Skipping binary as it could not be symlinked"
            );

            return Ok(false);
        }

        result.map_err(|error| {
            ProtoLayoutError::Fs(Box::new(FsError::Create {
                path: src_path.to_path_buf(),
                error: Box::new(error),
            }))
        })?;

        Ok(true)
    }

    /// Verify that symlinks can be created in the bin directory.
    #[instrument(skip(self))]
    pub fn check_bin_linking(&self) -> io::Result<()> {
        std::fs::create_dir_all(&self.bin_dir)?;

        let link_path = self
            .bin_dir
            .join(format!(".link-check-{}", std::process::id()));

        // The target doesn't need to exist
        symlink_file(&self.bin_dir.join(".link-check-target"), &link_path)?;

        std::fs::remove_file(&link_path)
    }

    #[instrument(skip(self))]
    pub fn unlink_bin(&self, bin_path: &Path) -> Result<(), ProtoLayoutError> {
        // Remove any file at this path, whether a symlink or not!
        if bin_path.is_symlink() {
            fs::remove_link(bin_path)?;
        } else if bin_path.is_file() {
            fs::remove_file(bin_path)?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn create_shim(&self, shim_path: &Path) -> Result<(), ProtoLayoutError> {
        create_shim(self.locate_shim_binary()?, shim_path).map_err(|error| {
            ProtoLayoutError::FailedCreateShim {
                path: shim_path.to_owned(),
                error: Box::new(error),
            }
        })?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn remove_shim(&self, shim_path: &Path) -> Result<(), ProtoLayoutError> {
        fs::remove(shim_path)?;

        Ok(())
    }
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store")
            .field("dir", &self.dir)
            .field("bin_dir", &self.bin_dir)
            .field("cache_dir", &self.cache_dir)
            .field("inventory_dir", &self.inventory_dir)
            .field("plugins_dir", &self.plugins_dir)
            .field("plugins_temp_dir", &self.plugins_temp_dir)
            .field("shims_dir", &self.shims_dir)
            .field("temp_dir", &self.temp_dir)
            .finish()
    }
}
