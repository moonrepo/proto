use crate::errors::ProtoError;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use starbase_utils::{
    fs::{self, FsError},
    json::{self, JsonError},
};
use std::{
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

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ManifestVersion {
    pub no_clean: bool,
    pub installed_at: u128,
    pub last_used_at: Option<u128>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Manifest {
    pub aliases: FxHashMap<String, String>,
    pub default_version: Option<String>,
    pub installed_versions: FxHashSet<String>,
    pub versions: FxHashMap<String, ManifestVersion>,

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

        manifest.versions.insert(
            version.to_owned(),
            ManifestVersion {
                installed_at: now(),
                no_clean: env::var("PROTO_NO_CLEAN").is_ok(),
                ..ManifestVersion::default()
            },
        );

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

        manifest.versions.remove(version);

        manifest.save()?;

        Ok(())
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoError> {
        Self::load(dir.as_ref().join(MANIFEST_NAME))
    }

    #[tracing::instrument(skip_all)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ProtoError> {
        let path = path.as_ref();

        debug!(path = %path.display(), "Loading manifest");

        let mut manifest: Manifest = if path.exists() {
            use fs4::FileExt;

            let file = fs::open_file(path)?;

            file.lock_shared().map_err(|error| FsError::Read {
                path: path.to_path_buf(),
                error,
            })?;

            let data = json::read_file(path)?;

            file.unlock().map_err(|error| FsError::Read {
                path: path.to_path_buf(),
                error,
            })?;

            data
        } else {
            Manifest::default()
        };

        manifest.path = path.to_owned();

        Ok(manifest)
    }

    #[tracing::instrument(skip_all)]
    pub fn save(&self) -> Result<(), ProtoError> {
        use fs4::FileExt;
        use std::io::prelude::*;

        debug!(path = %self.path.display(), "Saving manifest");

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::create_file(&self.path)?;

        file.lock_exclusive().map_err(|error| FsError::Write {
            path: self.path.to_path_buf(),
            error,
        })?;

        let data = json::to_string_pretty(&self).map_err(|error| JsonError::StringifyFile {
            path: self.path.to_path_buf(),
            error,
        })?;

        write!(file, "{}", data).unwrap();

        file.unlock().map_err(|error| FsError::Write {
            path: self.path.to_path_buf(),
            error,
        })?;

        Ok(())
    }

    pub fn track_used_at(&mut self, version: &str) {
        self.versions
            .entry(version.to_owned())
            .and_modify(|v| {
                v.last_used_at = Some(now());
            })
            .or_insert(ManifestVersion {
                last_used_at: Some(now()),
                ..ManifestVersion::default()
            });
    }
}
