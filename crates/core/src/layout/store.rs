use super::inventory::Inventory;
use super::layout_error::ProtoLayoutError;
use crate::id::Id;
use crate::tool_manifest::ToolManifest;
use once_cell::sync::OnceCell;
use proto_pdk_api::ToolInventoryOptions;
use proto_shim::{create_shim, locate_proto_exe};
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::{envx, fs, path};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};

#[derive(Clone, Default, Serialize)]
pub struct Store {
    pub dir: PathBuf,
    pub backends_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub builders_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub inventory_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub shims_dir: PathBuf,
    pub temp_dir: PathBuf,

    #[serde(skip)]
    shim_binary: Arc<OnceCell<Vec<u8>>>,
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

        Self {
            dir: dir.to_path_buf(),
            backends_dir: dir.join("backends"),
            bin_dir: dir.join("bin"),
            builders_dir: dir.join("builders"),
            cache_dir: dir.join("cache"),
            inventory_dir: dir.join("tools"),
            plugins_dir: dir.join("plugins"),
            shims_dir: dir.join("shims"),
            temp_dir,
            shim_binary: Arc::new(OnceCell::new()),
        }
    }

    #[instrument(skip(self, config))]
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
    pub fn load_shim_binary(&self) -> Result<&Vec<u8>, ProtoLayoutError> {
        self.shim_binary.get_or_try_init(|| {
            Ok(fs::read_file_bytes(
                locate_proto_exe("proto-shim").ok_or_else(|| {
                    ProtoLayoutError::MissingShimBinary {
                        bin_dir: self.bin_dir.clone(),
                    }
                })?,
            )?)
        })
    }

    #[instrument(skip(self))]
    pub fn save_preferred_profile(&self, path: &Path) -> Result<(), ProtoLayoutError> {
        fs::write_file(
            self.dir.join("profile"),
            path.as_os_str().as_encoded_bytes(),
        )?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn link_bin(&self, bin_path: &Path, src_path: &Path) -> Result<(), ProtoLayoutError> {
        // Windows requires admin privileges to create soft/hard links,
        // so just copy the binary... annoying...
        #[cfg(windows)]
        {
            fs::copy_file(src_path, bin_path)?;
        }

        #[cfg(unix)]
        {
            use starbase_utils::fs::FsError;

            std::os::unix::fs::symlink(src_path, bin_path).map_err(|error| {
                ProtoLayoutError::Fs(Box::new(FsError::Create {
                    path: src_path.to_path_buf(),
                    error: Box::new(error),
                }))
            })?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn unlink_bin(&self, bin_path: &Path) -> Result<(), ProtoLayoutError> {
        // Windows copies files
        #[cfg(windows)]
        fs::remove_file(bin_path)?;

        // Unix uses symlinks
        #[cfg(unix)]
        fs::remove_link(bin_path)?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn create_shim(&self, shim_path: &Path) -> Result<(), ProtoLayoutError> {
        create_shim(self.load_shim_binary()?, shim_path).map_err(|error| {
            ProtoLayoutError::FailedCreateShim {
                path: shim_path.to_owned(),
                error: Box::new(error),
            }
        })?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn remove_shim(&self, shim_path: &Path) -> Result<(), ProtoLayoutError> {
        fs::remove_file(shim_path)?;

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
            .field("shims_dir", &self.shims_dir)
            .field("temp_dir", &self.temp_dir)
            .finish()
    }
}
