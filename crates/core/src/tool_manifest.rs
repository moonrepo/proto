use crate::{
    read_json_file_with_lock,
    version::{AliasOrVersion, VersionType},
    write_json_file_with_lock,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use starbase_utils::fs;
use std::{
    collections::{BTreeMap, HashSet},
    env,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tracing::{debug, info};

fn now() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub const MANIFEST_NAME: &str = "manifest.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ToolManifestVersion {
    pub no_clean: bool,
    pub installed_at: u128,
    pub last_used_at: Option<u128>,
}

impl Default for ToolManifestVersion {
    fn default() -> Self {
        Self {
            no_clean: env::var("PROTO_NO_CLEAN").is_ok(),
            installed_at: now(),
            last_used_at: None,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ToolManifest {
    pub aliases: BTreeMap<String, VersionType>,
    pub default_version: Option<AliasOrVersion>,
    pub installed_versions: HashSet<Version>,
    pub shim_version: u8,
    pub versions: BTreeMap<Version, ToolManifestVersion>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl ToolManifest {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        Self::load(dir.as_ref().join(MANIFEST_NAME))
    }

    #[tracing::instrument(skip_all)]
    pub fn load<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading {}", MANIFEST_NAME);

        let mut manifest: ToolManifest = if path.exists() {
            read_json_file_with_lock(path)?
        } else {
            ToolManifest::default()
        };

        manifest.path = path.to_owned();

        Ok(manifest)
    }

    #[tracing::instrument(skip_all)]
    pub fn save(&self) -> miette::Result<()> {
        debug!(file = ?self.path, "Saving manifest");

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        write_json_file_with_lock(&self.path, self)?;

        Ok(())
    }

    pub fn insert_version(
        &mut self,
        version: &Version,
        default_version: Option<AliasOrVersion>,
    ) -> miette::Result<()> {
        if self.default_version.is_none() {
            self.default_version = Some(
                default_version.unwrap_or_else(|| AliasOrVersion::Version(version.to_owned())),
            );
        }

        self.installed_versions.insert(version.to_owned());

        self.versions
            .insert(version.to_owned(), ToolManifestVersion::default());

        self.save()?;

        Ok(())
    }

    pub fn remove_version(&mut self, version: &Version) -> miette::Result<()> {
        self.installed_versions.remove(version);

        // Remove default version if nothing available
        if (self.installed_versions.is_empty() && self.default_version.is_some())
            || self.default_version.as_ref().is_some_and(|v| v == version)
        {
            info!("Unpinning default global version");

            self.default_version = None;
        }

        self.versions.remove(version);

        self.save()?;

        Ok(())
    }

    pub fn track_used_at(&mut self, version: &Version) {
        self.versions
            .entry(version.to_owned())
            .and_modify(|v| {
                v.last_used_at = Some(now());
            })
            .or_insert(ToolManifestVersion {
                last_used_at: Some(now()),
                ..ToolManifestVersion::default()
            });
    }
}
