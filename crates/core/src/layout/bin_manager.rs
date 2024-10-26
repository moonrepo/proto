use crate::tool_manifest::ToolManifest;
use rustc_hash::{FxHashMap, FxHashSet};
use semver::Version;
use std::mem;
use version_spec::VersionSpec;

#[derive(Debug, Default)]
pub struct BinManager {
    buckets: FxHashMap<String, Version>,
    versions: FxHashSet<Version>,
}

impl BinManager {
    pub fn from_manifest(manifest: &ToolManifest) -> Self {
        let mut manager = Self::default();

        for spec in &manifest.installed_versions {
            if let Some(version) = spec.as_version() {
                manager.add_version(version);
            }
        }

        manager
    }

    pub fn get_buckets(&self) -> FxHashMap<&String, &Version> {
        self.buckets.iter().collect()
    }

    pub fn get_buckets_focused_to_version(
        &self,
        version: &Version,
    ) -> FxHashMap<&String, &Version> {
        let bucket_keys = self.get_keys(version);

        self.buckets
            .iter()
            .filter(|(key, _)| bucket_keys.contains(key))
            .collect()
    }

    pub fn add_version(&mut self, version: &Version) {
        for bucket_key in self.get_keys(version) {
            if let Some(bucket_value) = self.buckets.get_mut(&bucket_key) {
                // Always use the highest patch version
                if version > bucket_value {
                    *bucket_value = version.to_owned();
                }
            } else {
                self.buckets.insert(bucket_key.clone(), version.to_owned());
            }
        }

        self.versions.insert(version.to_owned());
    }

    pub fn rebuild_buckets(&mut self) {
        self.buckets.clear();

        for version in mem::take(&mut self.versions) {
            self.add_version(&version);
        }
    }

    pub fn remove_version(&mut self, version: &Version) -> bool {
        let mut rebuild = false;

        for bucket_key in self.get_keys(version) {
            if self
                .buckets
                .get(&bucket_key)
                .is_some_and(|bucket_value| bucket_value == version)
            {
                rebuild = true;
            }
        }

        self.versions.remove(version);

        if rebuild {
            self.rebuild_buckets();
        }

        rebuild
    }

    pub fn remove_version_from_spec(&mut self, spec: &VersionSpec) -> bool {
        if let Some(version) = spec.as_version() {
            return self.remove_version(version);
        }

        false
    }

    fn get_keys(&self, version: &Version) -> Vec<String> {
        vec![
            "*".to_string(),
            format!("{}", version.major),
            format!("{}.{}", version.major, version.minor),
        ]
    }
}
