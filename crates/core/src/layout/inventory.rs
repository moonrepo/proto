use super::product::Product;
use crate::helpers::{is_cache_enabled, is_offline};
use crate::tool_manifest::ToolManifest;
use proto_pdk_api::{LoadVersionsOutput, ToolInventoryMetadata};
use starbase_utils::{fs, json};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use version_spec::VersionSpec;

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub config: ToolInventoryMetadata,
    pub dir: PathBuf,
    pub manifest: ToolManifest,
    pub temp_dir: PathBuf,
}

impl Inventory {
    pub fn create_product(&self, spec: &VersionSpec) -> Product {
        let mut name = spec.to_string();

        if let Some(suffix) = &self.config.version_suffix {
            name = format!("{}{}", name, suffix);
        }

        Product {
            dir: self.dir.join(name),
            version: spec.to_owned(),
        }
    }

    pub fn load_remote_versions(
        &self,
        disable_cache: bool,
    ) -> miette::Result<Option<LoadVersionsOutput>> {
        let cache_path = self.dir.join("remote-versions.json");

        // Attempt to read from the cache first
        if cache_path.exists() {
            let mut read_cache =
                // Check if cache is enabled here, so that we can handle offline below
                if disable_cache || !is_cache_enabled() {
                    false
                // Otherwise, only read the cache every 12 hours
                } else {
                    let metadata = fs::metadata(&cache_path)?;

                    if let Ok(modified_time) = metadata.modified().or_else(|_| metadata.created()) {
                        modified_time > SystemTime::now() - Duration::from_secs(60 * 60 * 12)
                    } else {
                        false
                    }
                };

            // If offline, always read the cache
            if !read_cache && is_offline() {
                read_cache = true;
            }

            if read_cache {
                return Ok(Some(json::read_file(&cache_path)?));
            }
        }

        Ok(None)
    }

    pub fn save_remote_versions(&self, data: &LoadVersionsOutput) -> miette::Result<()> {
        json::write_file(self.dir.join("remote-versions.json"), data, false)?;

        Ok(())
    }
}
