use super::layout_error::ProtoLayoutError;
use crate::helpers::{read_json_file_with_lock, write_json_file_with_lock};
use crate::tool_context::ToolContext;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, warn};

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
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
    pub shims: ShimsMap,
    pub path: PathBuf,
    dirty: bool,
}

impl ShimRegistry {
    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoLayoutError> {
        Self::load(dir.as_ref().join("registry.json"))
    }

    #[instrument(name = "load_shim_registry")]
    pub fn load<P: AsRef<Path> + Debug>(path: P) -> Result<Self, ProtoLayoutError> {
        let path = path.as_ref();

        debug!(file = ?path, "Loading shims registry");

        let shims: ShimsMap = if path.exists() {
            read_json_file_with_lock(path)?
        } else {
            ShimsMap::default()
        };

        Ok(Self {
            shims,
            path: path.to_path_buf(),
            dirty: false,
        })
    }

    #[instrument(name = "update_shim_registry", skip(self))]
    pub fn update(&mut self, key: String, value: Shim) -> Result<(), ProtoLayoutError> {
        if let Some(current) = self.shims.get(&key) {
            // Don't write the file if nothing has changed
            if current == &value {
                return Ok(());
            }

            // A different tool already owns this executable name. Apply
            // "the primary tool owns its name" precedence so the outcome
            // doesn't depend on install order.
            match detect_shim_conflict(&key, &value, current) {
                Some(ShimConflict::Ignored { owner, provider }) => {
                    warn!(
                        shim = key.as_str(),
                        owner = owner.as_str(),
                        provider = provider.as_str(),
                        "Shim {} is already provided by {}, ignoring the duplicate from {}",
                        color::file(&key),
                        color::id(&owner),
                        color::id(&provider),
                    );

                    return Ok(());
                }
                Some(ShimConflict::Reclaimed { provider }) => {
                    debug!(
                        shim = key.as_str(),
                        provider = provider.as_str(),
                        "Shim {} reclaimed by its owning tool from {}",
                        color::file(&key),
                        color::id(&provider)
                    );
                }
                None => {}
            }
        }

        self.shims.insert(key, value);
        self.dirty = true;

        Ok(())
    }

    #[instrument(name = "save_shim_registry", skip(self))]
    pub fn save(&self) -> Result<(), ProtoLayoutError> {
        if self.dirty {
            debug!(file = ?self.path, "Saving shim registry");

            write_json_file_with_lock(&self.path, &self.shims)?;
        }

        Ok(())
    }
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
