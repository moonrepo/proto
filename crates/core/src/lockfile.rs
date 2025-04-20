use crate::tool_spec::Backend;
use proto_pdk_api::Checksum;
use serde::{Deserialize, Serialize};
use starbase_utils::toml;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};
use version_spec::VersionSpec;
use warpgate::Id;

pub const PROTO_LOCKFILE_NAME: &str = ".protolock";

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockfileRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,

    // Build from source and native installs may not have a checksum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    // Resolved version, only used in directory lockfiles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<VersionSpec>,
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
pub struct ProtoLockfile {
    pub tools: BTreeMap<Id, LockfileRecord>,
}

impl ProtoLockfile {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        let dir = dir.as_ref();

        Self::load(if dir.ends_with(PROTO_LOCKFILE_NAME) {
            dir.to_path_buf()
        } else {
            dir.join(PROTO_LOCKFILE_NAME)
        })
    }

    #[instrument(name = "load_lockfile")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> miette::Result<Self> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading lockfile");

        Ok(toml::read_file(path)?)
    }

    #[instrument(name = "save_lockfile", skip(lockfile))]
    pub fn save_to<P: AsRef<Path> + Debug>(
        dir: P,
        lockfile: ProtoLockfile,
    ) -> miette::Result<PathBuf> {
        let path = dir.as_ref();
        let file = if path.ends_with(PROTO_LOCKFILE_NAME) {
            path.to_path_buf()
        } else {
            path.join(PROTO_LOCKFILE_NAME)
        };

        toml::write_file(&file, &lockfile, true)?;

        Ok(file)
    }
}
