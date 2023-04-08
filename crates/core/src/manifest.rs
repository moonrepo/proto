use crate::errors::ProtoError;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::{fs, json};
use std::path::{Path, PathBuf};
use tracing::{info, trace};

pub const MANIFEST_NAME: &str = "manifest.json";

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Manifest {
    pub aliases: FxHashMap<String, String>,
    pub default_version: Option<String>,
    pub installed_versions: FxHashSet<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl Manifest {
    pub fn insert_version(
        path: PathBuf,
        version: &str,
        default_version: Option<&str>,
    ) -> Result<(), ProtoError> {
        let mut manifest = Manifest::load(path)?;

        if manifest.default_version.is_none() {
            manifest.default_version = Some(default_version.unwrap_or(version).to_owned());
        }

        manifest.installed_versions.insert(version.to_owned());
        manifest.save()?;

        Ok(())
    }

    pub fn remove_version(path: PathBuf, version: &str) -> Result<(), ProtoError> {
        let mut manifest = Manifest::load(path)?;

        manifest.installed_versions.remove(version);

        // Remove default version if nothing available
        if (manifest.installed_versions.is_empty() && manifest.default_version.is_some())
            || manifest.default_version.as_ref() == Some(&version.to_owned())
        {
            info!("Unpinning default global version");

            manifest.default_version = None;
        }

        manifest.save()?;

        Ok(())
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoError> {
        Self::load(dir.as_ref().join(MANIFEST_NAME))
    }

    #[tracing::instrument(skip_all)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ProtoError> {
        let path = path.as_ref();

        trace!("Loading manifest {}", color::path(path));

        let mut manifest: Manifest = if path.exists() {
            json::read_file(path)?
        } else {
            Manifest::default()
        };

        manifest.path = path.to_owned();

        Ok(manifest)
    }

    #[tracing::instrument(skip_all)]
    pub fn save(&self) -> Result<(), ProtoError> {
        trace!("Saving manifest {}", color::path(&self.path));

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        json::write_file(&self.path, self, true)?;

        Ok(())
    }
}
