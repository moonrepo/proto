use crate::tool_manifest::ToolManifest;
use rustc_hash::{FxHashMap, FxHashSet};
use std::mem;
use version_spec::VersionSpec;

/// Bucket key representing the highest installed version across all majors.
/// Resolves to the unversioned binary (e.g. `tool`).
pub const LATEST_BUCKET: &str = "*";

/// Bucket key for canary versions. Resolves to `tool-canary`.
pub const CANARY_BUCKET: &str = "canary";

/// Manages the versioned binaries that proto creates in `~/.proto/bin` for a
/// single tool. Each installed version is sorted into "buckets" keyed by a
/// partial version ([`LATEST_BUCKET`], `<major>`, `<major>.<minor>`, or
/// [`CANARY_BUCKET`]). Every bucket resolves to the highest installed version
/// that satisfies it, and maps directly to a binary file name on disk
/// (`tool`, `tool-1`, `tool-1.2`, `tool-canary`, ...).
#[derive(Clone, Debug, Default)]
pub struct BinManager {
    buckets: FxHashMap<String, VersionSpec>,
    versions: FxHashSet<VersionSpec>,
}

impl BinManager {
    /// Create a manager seeded with every version in the provided manifest.
    pub fn from_manifest(manifest: &ToolManifest) -> Self {
        let mut manager = Self::default();

        for spec in &manifest.installed_versions {
            manager.add_version(spec);
        }

        manager
    }

    /// Return all buckets and the version each currently resolves to.
    pub fn get_buckets(&self) -> FxHashMap<&String, &VersionSpec> {
        self.buckets.iter().collect()
    }

    /// Return only the buckets that the provided version participates in.
    pub fn get_buckets_focused_to_version(
        &self,
        spec: &VersionSpec,
    ) -> FxHashMap<&String, &VersionSpec> {
        let bucket_keys = Self::get_keys(spec);

        self.buckets
            .iter()
            .filter(|(key, _)| bucket_keys.contains(key))
            .collect()
    }

    /// Register a version, pointing each bucket it satisfies to it when it is
    /// higher than the bucket's current occupant. Aliases are ignored, as they
    /// are pointers to other versions rather than real installs.
    pub fn add_version(&mut self, spec: &VersionSpec) {
        if matches!(spec, VersionSpec::Alias(_)) {
            return;
        }

        for bucket_key in Self::get_keys(spec) {
            if let Some(bucket_value) = self.buckets.get_mut(&bucket_key) {
                // Always keep the highest version in each bucket
                if spec > bucket_value {
                    *bucket_value = spec.to_owned();
                }
            } else {
                self.buckets.insert(bucket_key, spec.to_owned());
            }
        }

        self.versions.insert(spec.to_owned());
    }

    /// Recompute all buckets from the current set of known versions.
    pub fn rebuild_buckets(&mut self) {
        self.buckets.clear();

        for version in mem::take(&mut self.versions) {
            self.add_version(&version);
        }
    }

    /// Remove a version from the set. Returns `true` if it occupied (was the
    /// highest version in) one or more buckets, in which case the buckets are
    /// rebuilt so they resolve to the next highest version.
    pub fn remove_version(&mut self, spec: &VersionSpec) -> bool {
        let mut rebuild = false;

        for bucket_key in Self::get_keys(spec) {
            if self
                .buckets
                .get(&bucket_key)
                .is_some_and(|bucket_value| bucket_value == spec)
            {
                rebuild = true;
            }
        }

        self.versions.remove(spec);

        if rebuild {
            self.rebuild_buckets();
        }

        rebuild
    }

    /// Return the bucket keys that the provided version participates in. Canary
    /// versions only belong to the canary bucket, aliases belong to none, and
    /// concrete versions belong to the latest, major, and minor buckets.
    fn get_keys(spec: &VersionSpec) -> Vec<String> {
        let mut keys = vec![];

        if spec.is_canary() {
            keys.push(CANARY_BUCKET.to_string());
        } else if let Some(version) = spec.as_version() {
            keys.extend([
                LATEST_BUCKET.to_string(),
                format!("{}", version.major),
                format!("{}.{}", version.major, version.minor),
            ]);
        }

        keys
    }
}
