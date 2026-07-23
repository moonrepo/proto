use super::layout_error::ProtoLayoutError;
use crate::helpers::write_json_file_atomic;
use crate::tool_context::ToolContext;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use starbase_utils::fs;
use starbase_utils::json::{self, JsonError};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, warn};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Shim {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after_args: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "alt_bin")]
    pub alt_exe: Option<bool>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before_args: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none", alias = "parent")]
    pub context: Option<ToolContext>,

    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub env_vars: FxHashMap<String, String>,
}

pub type ShimsMap = BTreeMap<String, Shim>;

pub struct ShimRegistry {
    // Access via tests only!
    pub shims: ShimsMap,

    changed: ShimsMap,
    path: PathBuf,
}

impl ShimRegistry {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoLayoutError> {
        Self::load(dir.as_ref().join("registry.json"))
    }

    #[instrument(name = "load_shim_registry")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> Result<Self, ProtoLayoutError> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading shims registry");

        Ok(Self {
            shims: read_shims_map(path)?,
            path: path.to_path_buf(),
            changed: ShimsMap::default(),
        })
    }

    pub fn get(&self, key: &str) -> Option<&Shim> {
        self.changed.get(key).or_else(|| self.shims.get(key))
    }

    #[instrument(name = "update_shim_registry", skip(self))]
    pub fn update(&mut self, key: String, value: Shim) -> Result<(), ProtoLayoutError> {
        if apply_shim_update(&mut self.shims, &key, &value) {
            self.changed.insert(key, value);
        }

        Ok(())
    }

    #[instrument(name = "save_shim_registry", skip(self))]
    pub fn save(&mut self) -> Result<(), ProtoLayoutError> {
        if self.changed.is_empty() {
            return Ok(());
        }

        debug!(file = ?self.path, "Saving shim registry");

        // The registry is a single file shared by every tool, while installs
        // may run concurrently, both in-process and across processes. Hold an
        // exclusive lock and merge our updates into a fresh read of the file,
        // so that a stale in-memory snapshot can't erase entries written by
        // another install since we loaded.
        // https://github.com/moonrepo/proto/issues/1057
        let _lock = fs::lock_file(self.path.with_extension("lock"))?;

        let mut shims = read_shims_map(&self.path)?;
        let mut dirty = false;

        for (key, value) in &self.changed {
            if apply_shim_update(&mut shims, key, value) {
                dirty = true;
            }
        }

        if dirty {
            write_json_file_atomic(&self.path, &shims)?;
        }

        self.shims = shims;
        self.changed.clear();

        Ok(())
    }
}

/// Read the shims map from the registry file. The lock taken here doesn't
/// guard the outer read-modify-write cycle (see [`ShimRegistry::save`] for
/// that), it only serializes against older proto binaries that still truncate
/// the file in place while holding an exclusive lock on it.
fn read_shims_map(path: &Path) -> Result<ShimsMap, ProtoLayoutError> {
    if !path.exists() {
        return Ok(ShimsMap::default());
    }

    let content = fs::read_file_with_lock(path)?;

    // An empty file can be left behind when a process is killed mid-write by
    // an older proto binary (truncate-then-write), or by external tampering.
    // Treat it as an empty registry rather than erroring: writes go through
    // `ShimRegistry::save`, which merges into a fresh read of the file, so
    // this can no longer silently wipe entries owned by other tools.
    if content.trim().is_empty() {
        warn!(
            file = ?path,
            "Shims registry file is unexpectedly empty, possibly from an interrupted write; treating it as an empty registry"
        );

        return Ok(ShimsMap::default());
    }

    let shims: ShimsMap =
        json::serde_json::from_str(&content).map_err(|error| JsonError::ReadFile {
            path: path.to_path_buf(),
            error: Box::new(error),
        })?;

    Ok(shims)
}

/// Apply a single shim entry to the map, enforcing "the primary tool owns its
/// name" precedence so the outcome doesn't depend on install order. Returns
/// true if the map was modified.
fn apply_shim_update(shims: &mut ShimsMap, key: &str, value: &Shim) -> bool {
    if let Some(current) = shims.get(key) {
        // Don't write the file if nothing has changed
        if current == value {
            return false;
        }

        // A different tool already owns this executable name.
        match detect_shim_conflict(key, value, current) {
            Some(ShimConflict::Ignored { owner, provider }) => {
                warn!(
                    shim = key,
                    owner = owner.as_str(),
                    provider = provider.as_str(),
                    "Shim {} is already provided by {}, ignoring the duplicate from {}",
                    color::file(key),
                    color::id(&owner),
                    color::id(&provider),
                );

                return false;
            }
            Some(ShimConflict::Reclaimed { provider }) => {
                debug!(
                    shim = key,
                    provider = provider.as_str(),
                    "Shim {} reclaimed by its owning tool from {}",
                    color::file(key),
                    color::id(&provider)
                );
            }
            None => {}
        }
    }

    shims.insert(key.to_owned(), value.clone());
    true
}

/// A cross-tool conflict detected while updating the shims registry. An entry's
/// owner is the tool referenced by its `context`, or — when `context` is `None`
/// — the primary tool whose id matches the executable name.
enum ShimConflict {
    /// The incoming executable loses to the tool that already owns the name and
    /// should be ignored.
    Ignored { owner: String, provider: String },
    /// A primary tool reclaims its own name from a prior secondary provider.
    Reclaimed { provider: String },
}

/// Determine whether an incoming shim entry conflicts with the existing one,
/// applying "the primary tool owns its name" precedence. Returns `None` when
/// both entries resolve to the same owner (no conflict).
fn detect_shim_conflict(name: &str, incoming: &Shim, existing: &Shim) -> Option<ShimConflict> {
    match (&incoming.context, &existing.context) {
        // A secondary executable can't take a name owned by a primary tool.
        (Some(provider), None) => Some(ShimConflict::Ignored {
            owner: name.to_owned(),
            provider: provider.as_str().to_owned(),
        }),
        // Two different tools provide the same secondary executable; first wins.
        (Some(provider), Some(owner)) if provider != owner => Some(ShimConflict::Ignored {
            owner: owner.as_str().to_owned(),
            provider: provider.as_str().to_owned(),
        }),
        // The primary tool reclaims its name from a prior secondary provider.
        (None, Some(provider)) => Some(ShimConflict::Reclaimed {
            provider: provider.as_str().to_owned(),
        }),
        // Same owner, or both the primary of this name.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;

    fn primary() -> Shim {
        Shim::default()
    }

    fn secondary(tool: &str) -> Shim {
        Shim {
            context: Some(ToolContext::new(Id::raw(tool))),
            ..Default::default()
        }
    }

    #[test]
    fn no_conflict_between_same_primary() {
        assert!(detect_shim_conflict("go", &primary(), &primary()).is_none());
    }

    #[test]
    fn no_conflict_between_same_secondary_owner() {
        assert!(detect_shim_conflict("dlv", &secondary("go"), &secondary("go")).is_none());
    }

    #[test]
    fn secondary_loses_to_existing_primary() {
        assert!(matches!(
            detect_shim_conflict("go", &secondary("xyz"), &primary()),
            Some(ShimConflict::Ignored { owner, provider }) if owner == "go" && provider == "xyz"
        ));
    }

    #[test]
    fn secondary_loses_to_existing_secondary() {
        assert!(matches!(
            detect_shim_conflict("foo", &secondary("b"), &secondary("a")),
            Some(ShimConflict::Ignored { owner, provider }) if owner == "a" && provider == "b"
        ));
    }

    #[test]
    fn primary_reclaims_from_secondary() {
        assert!(matches!(
            detect_shim_conflict("go", &primary(), &secondary("xyz")),
            Some(ShimConflict::Reclaimed { provider }) if provider == "xyz"
        ));
    }
}
