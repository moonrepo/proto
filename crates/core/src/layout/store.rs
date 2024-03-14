use super::product::Product;
use rustc_hash::FxHashMap;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use warpgate::Id;

#[derive(Clone)]
pub struct Store {
    pub dir: PathBuf,
    pub bin_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub shims_dir: PathBuf,
    pub temp_dir: PathBuf,

    pub products_dir: PathBuf,
    pub products: FxHashMap<Id, Product>,
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
            products: FxHashMap::default(),
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

    pub fn save_preferred_profile(&self, path: &Path) -> miette::Result<()> {
        fs::write_file(
            self.dir.join("profile"),
            path.as_os_str().as_encoded_bytes(),
        )?;

        Ok(())
    }
}
