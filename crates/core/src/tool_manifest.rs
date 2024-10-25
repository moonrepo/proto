use crate::helpers::{now, read_json_file_with_lock, write_json_file_with_lock};
use rustc_hash::{FxHashMap, FxHashSet};
use semver::Version;
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
    pub no_clean: bool,
    pub installed_at: u128,
}

impl Default for ToolManifestVersion {
    fn default() -> Self {
        Self {
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

    pub fn get_bucketed_versions(
        &self,
        focused_version: Option<&Version>,
    ) -> FxHashMap<String, Version> {
        let mut versions = FxHashMap::default();

        let get_keys = |version: &Version| -> Vec<String> {
            vec![
                format!("{}", version.major),
                format!("{}.{}", version.major, version.minor),
            ]
        };

        let mut add = |version: &Version| {
            for bucket_key in get_keys(version) {
                if let Some(bucket_value) = versions.get_mut(&bucket_key) {
                    // Always use the highest patch version
                    if version > bucket_value {
                        *bucket_value = version.to_owned();
                    }
                } else {
                    versions.insert(bucket_key.clone(), version.to_owned());
                }
            }
        };

        for spec in &self.installed_versions {
            if let Some(version) = spec.as_version() {
                add(version);
            }
        }

        // If we have a focused version, add it to the bucketed map,
        // and then filter down the map to the key with the same matching range.
        // This may result in a different patch version then the patch in the
        // focused version if there is an installed version with a higher patch.
        if let Some(version) = focused_version {
            add(version);

            let bucket_keys = get_keys(version);

            versions.retain(|key, _| bucket_keys.contains(key));
        }

        versions
    }
}
