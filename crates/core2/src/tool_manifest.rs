use crate::version::{AliasOrVersion, VersionType};
use rustc_hash::FxHashSet;
use semver::Version;
use serde::{Deserialize, Serialize};
use starbase_utils::{
    fs::{self, FsError},
    json::{self, JsonError},
};
use std::{
    collections::BTreeMap,
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
pub struct ToolManifestVersion {
    pub no_clean: bool,
    pub installed_at: u128,
    pub last_used_at: Option<u128>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ToolManifest {
    pub aliases: BTreeMap<String, VersionType>,
    pub default_version: Option<AliasOrVersion>,
    pub installed_versions: FxHashSet<Version>,
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
            ToolManifest::default()
        };

        manifest.path = path.to_owned();

        Ok(manifest)
    }

    #[tracing::instrument(skip_all)]
    pub fn save(&self) -> miette::Result<()> {
        use fs4::FileExt;
        use std::io::{prelude::*, SeekFrom};

        debug!(file = ?self.path, "Saving manifest");

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
        version: &Version,
        default_version: Option<AliasOrVersion>,
    ) -> miette::Result<()> {
        if self.default_version.is_none() {
            self.default_version = Some(
                default_version.unwrap_or_else(|| AliasOrVersion::Version(version.to_owned())),
            );
        }

        self.installed_versions.insert(version.to_owned());

        self.versions.insert(
            version.to_owned(),
            ToolManifestVersion {
                installed_at: now(),
                no_clean: env::var("PROTO_NO_CLEAN").is_ok(),
                ..ToolManifestVersion::default()
            },
        );

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
