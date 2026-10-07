use crate::id::Id;
use crate::tool_context::ToolContext;
use proto_pdk_api::{Checksum, ToolLockOptions};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_utils::fs;
use starbase_utils::toml::{self, TomlError};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use system_env::{SystemArch, SystemLibc, SystemOS, SystemPlatform};
use tracing::{debug, instrument};
use version_spec::{UnresolvedVersionSpec, VersionSpec};

pub const PROTO_LOCK_NAME: &str = ".protolock";

/// Return the libc to record in the lockfile for the provided platform. A libc
/// is only recorded for Linux, as it's the only operating system where artifacts
/// are commonly distributed for multiple libcs (GNU and musl).
pub fn get_lockable_libc(platform: &SystemPlatform) -> Option<SystemLibc> {
    (platform.os.is_linux() && platform.libc != SystemLibc::Unknown).then_some(platform.libc)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct LockRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<SystemOS>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<SystemArch>,

    /// Only set on Linux, where artifacts are distributed per libc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libc: Option<SystemLibc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<Id>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<UnresolvedVersionSpec>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<VersionSpec>,

    // Build from source and native installs may not have a checksum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Additional metadata to include in lockfile records for this tool
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub metadata: FxHashMap<String, String>,
}

impl LockRecord {
    pub fn for_manifest(&self) -> Self {
        let mut record = self.clone();
        record.spec = None;
        record.version = None;
        record
    }

    pub fn for_lockfile(&self) -> Self {
        let mut record = self.clone();
        record.source = None;
        record
    }

    /// Create a copy of this record that only retains the information that
    /// is valid on every operating system, architecture, and libc. The resolved
    /// version applies to all machines, while the checksum, source, and
    /// metadata are derived from a platform specific artifact, so they are
    /// removed, and are repopulated when the current platform installs.
    pub fn for_other_platform(&self) -> Self {
        let mut record = self.clone();
        record.os = None;
        record.arch = None;
        record.libc = None;
        record.checksum = None;
        record.source = None;
        record.metadata = FxHashMap::default();
        record
    }

    /// Set the operating system, architecture, and libc from the provided
    /// platform. The libc is only set for Linux, see [`get_lockable_libc`].
    pub fn set_platform(&mut self, platform: &SystemPlatform) {
        self.os = Some(platform.os);
        self.arch = Some(platform.arch);
        self.libc = get_lockable_libc(platform);
    }

    /// Return true if this record (in the lockfile) matches the other record,
    /// by comparing the backend, spec, and platform.
    pub fn is_match(&self, other: &Self, options: &ToolLockOptions) -> bool {
        if self.backend != other.backend || self.spec != other.spec {
            return false;
        }

        if options.ignore_os_arch {
            // If the tool is ignoring os/arch but this record (in the lockfile)
            // has an os/arch/libc, then it shouldn't match
            if self.os.is_some() || self.arch.is_some() || self.libc.is_some() {
                return false;
            }
        } else {
            // If the tool is matching os/arch, then we need to ensure that this
            // record (in the lockfile) matches the values, except for none,
            // as none entries exist for backwards compatibility
            if self.os.is_some() && self.os != other.os
                || self.arch.is_some() && self.arch != other.arch
                || self.libc.is_some() && self.libc != other.libc
            {
                return false;
            }
        }

        true
    }

    /// Return true if this record (in the lockfile) matches the provided
    /// backend, spec, and platform.
    pub fn is_match_with(
        &self,
        backend: Option<&Id>,
        spec: Option<&UnresolvedVersionSpec>,
        platform: &SystemPlatform,
        options: &ToolLockOptions,
    ) -> bool {
        let mut other = LockRecord {
            backend: backend.cloned(),
            spec: spec.cloned(),
            ..Default::default()
        };
        other.set_platform(platform);

        self.is_match(&other, options)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProtoLock {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub tools: BTreeMap<Id, Vec<LockRecord>>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl ProtoLock {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, TomlError> {
        Self::load(Self::resolve_path(dir))
    }

    #[instrument(name = "load_lock")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> Result<Self, TomlError> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading lock file");

        let mut manifest: ProtoLock = if path.exists() {
            toml::read_file(path)?
        } else {
            ProtoLock::default()
        };

        manifest.path = path.into();

        Ok(manifest)
    }

    #[instrument(name = "save_lock", skip(self))]
    pub fn save(&self) -> Result<(), TomlError> {
        if self.tools.is_empty() {
            debug!(file = ?self.path, "Removing lock file because its empty");

            fs::remove_file(&self.path)?;

            return Ok(());
        }

        debug!(file = ?self.path, "Saving lock file");

        let content = toml::format(self, true)?;

        fs::write_file(
            &self.path,
            format!("# Generated by proto. Do not modify!\n\n{content}"),
        )?;

        Ok(())
    }

    /// Remove records that have been orphaned by configuration changes.
    /// A record is orphaned when its tool has a version defined in the
    /// provided map of configured specifications, but the record's spec
    /// no longer matches any of them. Records for tools that are not in
    /// the map are ad-hoc installs, and are always kept. Records are
    /// removed across all operating systems and architectures, as a
    /// change in configuration applies to every machine.
    ///
    /// Returns the number of records that were removed.
    pub fn prune_orphaned_records(
        &mut self,
        config_specs: &BTreeMap<&ToolContext, BTreeSet<&UnresolvedVersionSpec>>,
    ) -> usize {
        let path = &self.path;
        let mut pruned = 0;

        self.tools.retain(|id, records| {
            records.retain(|record| {
                // Records without a spec exist for backwards compatibility,
                // and cannot be compared against the configuration
                let Some(record_spec) = &record.spec else {
                    return true;
                };

                // Only tools defined in the config that owns this lockfile
                // can be verified, as records may also exist for ad-hoc
                // installs, which are never configured
                let Some(specs) = config_specs.iter().find_map(|(context, specs)| {
                    (context.id == *id && context.backend == record.backend).then_some(specs)
                }) else {
                    return true;
                };

                if specs.contains(record_spec) {
                    return true;
                }

                debug!(
                    file = ?path,
                    tool = id.as_str(),
                    spec = record_spec.to_string(),
                    "Pruning orphaned record from lock file",
                );

                pruned += 1;
                false
            });

            // Remove the tool entirely when all of its records were pruned
            !records.is_empty()
        });

        pruned
    }

    pub fn sort_records(&mut self) {
        for records in self.tools.values_mut() {
            records.sort_by_key(|record| {
                (
                    record.spec.clone(),
                    record.backend.clone(),
                    record.os,
                    record.arch,
                    record.libc,
                )
            });
        }
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
