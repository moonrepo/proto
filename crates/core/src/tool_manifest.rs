use crate::helpers::{now, read_json_file_with_lock, write_json_file_with_lock};
use crate::tool_spec::Backend;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use starbase_utils::env::bool_var;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};
use tracing::{debug, instrument};
use version_spec::*;

pub const MANIFEST_NAME: &str = "manifest.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ToolManifestVersion {
    pub backend: Backend,
    pub no_clean: bool,
    pub installed_at: u128,
}

impl Default for ToolManifestVersion {
    fn default() -> Self {
        Self {
            backend: Backend::Proto,
            no_clean: bool_var("PROTO_NO_CLEAN"),
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

    #[instrument(name = "load_tool_manifest")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> miette::Result<Self> {
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

    #[instrument(name = "save_tool_manifest", skip(self))]
    pub fn save(&self) -> miette::Result<()> {
        debug!(file = ?self.path, "Saving manifest");

        write_json_file_with_lock(&self.path, self)?;

        Ok(())
    }
}
