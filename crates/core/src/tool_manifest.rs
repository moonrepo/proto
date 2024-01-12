use crate::helpers::{now, read_json_file_with_lock, write_json_file_with_lock};
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::fs;
use std::{
    collections::{BTreeMap, HashSet},
    env,
    path::{Path, PathBuf},
};
use tracing::{debug, warn};
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
    // Partial versions allowed
    #[deprecated]
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,
    #[deprecated]
    pub default_version: Option<UnresolvedVersionSpec>,

    // Full versions only
    pub installed_versions: HashSet<VersionSpec>,
    pub shim_version: u8,
    pub versions: BTreeMap<VersionSpec, ToolManifestVersion>,

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

        manifest.path = path.to_owned();

        if env::var("PROTO_IGNORE_MIGRATE_WARNING").is_err() {
            #[allow(deprecated)]
            if !manifest.aliases.is_empty() {
                warn!(
                    "Found legacy aliases in tool manifest, please run {} to migrate them",
                    color::shell("proto migrate v0.24")
                );
            }

            #[allow(deprecated)]
            if manifest.default_version.is_some() {
                warn!(
                    "Found legacy global version in tool manifest, please run {} to migrate it",
                    color::shell("proto migrate v0.24")
                );
            }
        }

        Ok(manifest)
    }

    pub fn save(&self) -> miette::Result<()> {
        debug!(file = ?self.path, "Saving manifest");

        write_json_file_with_lock(&self.path, self)?;

        Ok(())
    }

    pub fn track_used_at(&mut self, tool_dir: impl AsRef<Path>) -> miette::Result<()> {
        fs::write_file(tool_dir.as_ref().join(".last-used"), now().to_string())?;

        Ok(())
    }

    pub fn load_used_at(&self, tool_dir: impl AsRef<Path>) -> miette::Result<Option<u128>> {
        let file = tool_dir.as_ref().join(".last-used");

        if file.exists() {
            if let Ok(contents) = fs::read_file(file) {
                if let Ok(value) = contents.parse::<u128>() {
                    return Ok(Some(value));
                }
            }
        }

        Ok(None)
    }
}
