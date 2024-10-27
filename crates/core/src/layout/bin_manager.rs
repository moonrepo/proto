use crate::tool_manifest::ToolManifest;
use rustc_hash::{FxHashMap, FxHashSet};
use std::mem;
use version_spec::VersionSpec;

#[derive(Debug, Default)]
pub struct BinManager {
    buckets: FxHashMap<String, VersionSpec>,
    versions: FxHashSet<VersionSpec>,
}

impl BinManager {
    pub fn from_manifest(manifest: &ToolManifest) -> Self {
        let mut manager = Self::default();

        for spec in &manifest.installed_versions {
            manager.add_version(spec);
        }

        manager
    }

    pub fn get_buckets(&self) -> FxHashMap<&String, &VersionSpec> {
        self.buckets.iter().collect()
    }

    pub fn get_buckets_focused_to_version(
        &self,
        spec: &VersionSpec,
    ) -> FxHashMap<&String, &VersionSpec> {
        let bucket_keys = self.get_keys(spec);

        self.buckets
            .iter()
            .filter(|(key, _)| bucket_keys.contains(key))
            .collect()
    }

    pub fn add_version(&mut self, spec: &VersionSpec) {
        if matches!(spec, VersionSpec::Alias(_)) {
            return;
        }

        for bucket_key in self.get_keys(spec) {
            if bucket_key == "canary" && !spec.is_canary() {
                continue;
            }

            if let Some(bucket_value) = self.buckets.get_mut(&bucket_key) {
                // Always use the highest patch version
                if spec > bucket_value {
                    *bucket_value = spec.to_owned();
                }
            } else {
                self.buckets.insert(bucket_key.clone(), spec.to_owned());
            }
        }

        self.versions.insert(spec.to_owned());
    }

    pub fn rebuild_buckets(&mut self) {
        self.buckets.clear();

        for version in mem::take(&mut self.versions) {
            self.add_version(&version);
        }
    }

    pub fn remove_version(&mut self, spec: &VersionSpec) -> bool {
        let mut rebuild = false;

        for bucket_key in self.get_keys(spec) {
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

    fn get_keys(&self, spec: &VersionSpec) -> Vec<String> {
        let mut keys = vec![];

        if spec.is_canary() {
            keys.push("canary".to_string());
        } else if let Some(version) = spec.as_version() {
            keys.extend([
                "*".to_string(),
                format!("{}", version.major),
                format!("{}.{}", version.major, version.minor),
            ]);
        }

        keys
    }
}
