use crate::tool_spec::Backend;
use proto_pdk_api::Checksum;
use serde::{Deserialize, Serialize};
use starbase_utils::toml::{self, TomlError};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};
use version_spec::{UnresolvedVersionSpec, VersionSpec};
use warpgate::Id;

pub const PROTO_LOCK_NAME: &str = ".protolock";

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockfileRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<UnresolvedVersionSpec>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<VersionSpec>,

    // Build from source and native installs may not have a checksum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl LockfileRecord {
    pub fn new(backend: Option<Backend>) -> Self {
        Self {
            backend,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lockfile {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub plugins: BTreeMap<Id, LockfileRecord>,

    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<Id, LockfileRecord>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl Lockfile {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, TomlError> {
        Self::load(Self::resolve_path(dir))
    }

    #[instrument(name = "load_lock")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> Result<Self, TomlError> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading lockfile");

        let mut manifest: Lockfile = if path.exists() {
            toml::read_file(path)?
        } else {
            Lockfile::default()
        };

        manifest.path = path.into();

        Ok(manifest)
    }

    #[instrument(name = "save_lock", skip(self))]
    pub fn save(&self) -> Result<(), TomlError> {
        debug!(file = ?self.path, "Saving lockfile");

        toml::write_file(&self.path, self, true)?;

        Ok(())
    }

    fn resolve_path(path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();

        if path.ends_with(PROTO_LOCK_NAME) {
            path.to_path_buf()
        } else {
            path.join(PROTO_LOCK_NAME)
        }
    }
}
