use super::product::Product;
use crate::error::ProtoError;
use crate::tool_manifest::ToolManifest;
use miette::IntoDiagnostic;
use once_cell::sync::OnceCell;
use proto_shim::{create_shim, locate_proto_exe};
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use warpgate::Id;

#[derive(Clone)]
pub struct Store {
    pub dir: PathBuf,
    pub bin_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub products_dir: PathBuf,
    pub shims_dir: PathBuf,
    pub temp_dir: PathBuf,

    shim_binary: OnceCell<Vec<u8>>,
}

impl Store {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            bin_dir: dir.join("bin"),
            plugins_dir: dir.join("plugins_dir"),
            shims_dir: dir.join("shims"),
            temp_dir: dir.join("temp"),
            products_dir: dir.join("tools"),
            shim_binary: OnceCell::new(),
        }
    }

    pub fn load_uuid(&self) -> miette::Result<String> {
        let id_path = self.dir.join("id");

        if id_path.exists() {
            return Ok(fs::read_file(id_path)?);
        }

        let id = uuid::Uuid::new_v4().to_string();

        fs::write_file(id_path, &id)?;

        Ok(id)
    }

    pub fn load_preferred_profile(&self) -> miette::Result<Option<PathBuf>> {
        let profile_path = self.dir.join("profile");

        if profile_path.exists() {
            return Ok(Some(fs::read_file(profile_path)?.into()));
        }

        Ok(None)
    }

    pub fn load_product(&self, id: &Id) -> miette::Result<Product> {
        Ok(Product {
            dir: self.products_dir.join(id.as_str()),
            temp_dir: self.temp_dir.join(id.as_str()),
            manifest: ToolManifest::load_from(&self.dir)?,
        })
    }

    pub fn load_shim_binary(&self) -> miette::Result<&Vec<u8>> {
        self.shim_binary.get_or_try_init(|| {
            Ok(fs::read_file_bytes(
                locate_proto_exe("proto-shim").ok_or_else(|| ProtoError::MissingShimBinary {
                    bin_dir: self.bin_dir.clone(),
                })?,
            )?)
        })
    }

    pub fn save_preferred_profile(&self, path: &Path) -> miette::Result<()> {
        fs::write_file(
            self.dir.join("profile"),
            path.as_os_str().as_encoded_bytes(),
        )?;

        Ok(())
    }

    pub fn link_bin(&self, bin_path: &Path, src_path: &Path) -> miette::Result<()> {
        // Windows requires admin privileges to create soft/hard links,
        // so just copy the binary... annoying...
        #[cfg(windows)]
        fs::copy_file(src_path, bin_path)?;

        #[cfg(not(windows))]
        std::os::unix::fs::symlink(src_path, bin_path).into_diagnostic()?;

        Ok(())
    }

    pub fn unlink_bin(&self, bin_path: &Path) -> miette::Result<()> {
        // Windows copies files
        #[cfg(windows)]
        fs::remove_file(bin_path)?;

        // Unix uses symlinks
        #[cfg(not(windows))]
        fs::remove_link(bin_path)?;

        Ok(())
    }

    pub fn create_shim(&self, shim_path: &Path, find_only: bool) -> miette::Result<()> {
        create_shim(self.load_shim_binary()?, shim_path, find_only).map_err(|error| {
            ProtoError::CreateShimFailed {
                path: shim_path.to_owned(),
                error,
            }
        })?;

        Ok(())
    }

    pub fn remove_shim(&self, shim_path: &Path) -> miette::Result<()> {
        fs::remove_file(shim_path)?;

        Ok(())
    }
}
