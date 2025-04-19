use crate::helpers::{read_json_file_with_lock, write_json_file_with_lock};
use crate::lockfile::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};
use version_spec::VersionSpec;

pub const LOCKFILE_NAME: &str = "lockfile.json";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolLockfile {
    pub versions: BTreeMap<VersionSpec, LockfileRecord>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl ToolLockfile {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        Self::load(dir.as_ref().join(LOCKFILE_NAME))
    }

    #[instrument(name = "load_tool_lockfile")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> miette::Result<Self> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading lockfile");

        let mut manifest: ToolLockfile = if path.exists() {
            read_json_file_with_lock(path)?
        } else {
            ToolLockfile::default()
        };

        path.clone_into(&mut manifest.path);

        Ok(manifest)
    }

    #[instrument(name = "save_tool_lockfile", skip(self))]
    pub fn save(&self) -> miette::Result<()> {
        debug!(file = ?self.path, "Saving lockfile");

        write_json_file_with_lock(&self.path, self)?;

        Ok(())
    }
}
