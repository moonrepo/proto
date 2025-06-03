use crate::ProtoToolError;
use crate::helpers::{now, read_json_file_with_lock, write_json_file_with_lock};
use crate::lockfile::LockfileRecord;
use crate::tool_spec::Backend;
use serde::{Deserialize, Serialize};
use starbase_utils::env::bool_var;
use std::collections::{BTreeMap, BTreeSet};
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
    // TODO deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,

    pub no_clean: bool,

    pub installed_at: u128,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<LockfileRecord>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

impl Default for ToolManifestVersion {
    fn default() -> Self {
        Self {
            backend: None,
            no_clean: bool_var("PROTO_NO_CLEAN"),
            installed_at: now(),
            lock: None,
            suffix: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ToolManifest {
    pub installed_versions: BTreeSet<VersionSpec>,
    pub shim_version: u8,
    pub versions: BTreeMap<VersionSpec, ToolManifestVersion>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl ToolManifest {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoToolError> {
        Self::load(dir.as_ref().join(MANIFEST_NAME))
    }

    #[instrument(name = "load_tool_manifest")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> Result<Self, ProtoToolError> {
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
    pub fn save(&self) -> Result<(), ProtoToolError> {
        debug!(file = ?self.path, "Saving manifest");

        write_json_file_with_lock(&self.path, self)?;

        Ok(())
    }
}
