use crate::helpers::{now, read_json_file_with_lock, write_json_file_with_lock};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};
use tracing::debug;
use version_spec::*;

pub const MANIFEST_NAME: &str = "manifest.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ToolManifestVersion {
    pub no_clean: bool,
    pub installed_at: u128,
}

impl Default for ToolManifestVersion {
    fn default() -> Self {
        Self {
            no_clean: env::var("PROTO_NO_CLEAN").is_ok(),
            installed_at: now(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ToolManifest {
    pub installed_versions: FxHashSet<VersionSpec>,
    pub shim_version: u8,
    pub versions: FxHashMap<VersionSpec, ToolManifestVersion>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl ToolManifest {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        Self::load(dir.as_ref().join(MANIFEST_NAME))
    }

    pub fn load<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading {}", MANIFEST_NAME);

        let mut manifest: ToolManifest = if path.exists() {
            read_json_file_with_lock(path)?
        } else {
            ToolManifest::default()
        };

        path.clone_into(&mut manifest.path);

        Ok(manifest)
    }

    pub fn save(&self) -> miette::Result<()> {
        debug!(file = ?self.path, "Saving manifest");

        write_json_file_with_lock(&self.path, self)?;

        Ok(())
    }
}
