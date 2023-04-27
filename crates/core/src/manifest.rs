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
    pub shim_version: u8,
    pub versions: FxHashMap<String, ManifestVersion>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl Manifest {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoError> {
        Self::load(dir.as_ref().join(MANIFEST_NAME))
    }

    #[tracing::instrument(skip_all)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ProtoError> {
        let path = path.as_ref();

        debug!(file = %path.display(), "Loading manifest");

        let mut manifest: Manifest = if path.exists() {
            use fs4::FileExt;
            use std::io::prelude::*;

            let handle_error = |error: std::io::Error| FsError::Read {
                path: path.to_path_buf(),
                error,
            };

            let mut file = fs::open_file(path)?;
            let mut buffer = String::new();

            file.lock_shared().map_err(handle_error)?;
            file.read_to_string(&mut buffer).map_err(handle_error)?;

            let data = json::from_str(&buffer).map_err(|error| JsonError::ReadFile {
                path: path.to_path_buf(),
                error,
            })?;

            file.unlock().map_err(handle_error)?;

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
        use std::io::{prelude::*, SeekFrom};

        debug!(file = %self.path.display(), "Saving manifest");

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let handle_error = |error: std::io::Error| FsError::Write {
            path: self.path.to_path_buf(),
            error,
        };

        // Don't use fs::create_file() as it truncates!
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&self.path)
            .map_err(handle_error)?;

        file.lock_exclusive().map_err(handle_error)?;

        let data = json::to_string_pretty(&self).map_err(|error| JsonError::StringifyFile {
            path: self.path.to_path_buf(),
            error,
        })?;

        // Truncate then write file
        file.set_len(0).map_err(handle_error)?;
        file.seek(SeekFrom::Start(0)).map_err(handle_error)?;

        write!(file, "{}", data).map_err(handle_error)?;

        file.unlock().map_err(handle_error)?;

        Ok(())
    }

    pub fn insert_version(
        &mut self,
        version: &str,
        default_version: Option<String>,
    ) -> Result<(), ProtoError> {
        if self.default_version.is_none() {
            self.default_version = Some(default_version.unwrap_or(version.to_owned()));
        }

        self.installed_versions.insert(version.to_owned());

        self.versions.insert(
            version.to_owned(),
            ManifestVersion {
                installed_at: now(),
                no_clean: env::var("PROTO_NO_CLEAN").is_ok(),
                ..ManifestVersion::default()
            },
        );

        self.save()?;

        Ok(())
    }

    pub fn remove_version(&mut self, version: &str) -> Result<(), ProtoError> {
        self.installed_versions.remove(version);

        // Remove default version if nothing available
        if (self.installed_versions.is_empty() && self.default_version.is_some())
            || self.default_version.as_ref() == Some(&version.to_owned())
        {
            info!("Unpinning default global version");

            self.default_version = None;
        }

        self.versions.remove(version);

        self.save()?;

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
