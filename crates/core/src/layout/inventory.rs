use super::layout_error::ProtoLayoutError;
use super::product::Product;
use crate::helpers::{is_cache_enabled, is_offline};
use crate::lockfile::LockRecord;
use crate::tool_manifest::ToolManifest;
use proto_pdk_api::{LoadVersionsOutput, ToolInventoryOptions};
use starbase_utils::{fs, json, path};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tracing::instrument;
use version_spec::VersionSpec;

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub config: ToolInventoryOptions,
    pub dir: PathBuf,
    pub dir_original: Option<PathBuf>,
    pub manifest: ToolManifest,
    pub temp_dir: PathBuf,
}

impl Inventory {
    pub fn create_product(&self, version: &VersionSpec) -> Product {
        Product {
            dir: self.get_product_dir(version),
            version: version.to_owned(),
        }
    }

    pub fn get_locked_record(&self, version: &VersionSpec) -> Option<&LockRecord> {
        self.manifest
            .versions
            .get(version)
            .and_then(|man| man.lock.as_ref())
    }

    pub fn get_product_dir(&self, version: &VersionSpec) -> PathBuf {
        let mut name = version.to_string();

        if let Some(suffix) = &self.config.version_suffix {
            name = format!("{name}{suffix}");
        }

        self.dir.join(path::encode_component(name))
    }

    #[instrument(skip(self))]
    pub fn load_manifest(&self) -> Result<ToolManifest, ProtoLayoutError> {
        Ok(ToolManifest::load_from(
            self.dir_original.as_deref().unwrap_or(self.dir.as_ref()),
        )?)
    }

    #[instrument(skip(self))]
    pub fn load_remote_versions(
        &self,
        disable_cache: bool,
    ) -> Result<Option<LoadVersionsOutput>, ProtoLayoutError> {
        let cache_path = self
            .dir_original
            .as_ref()
            .unwrap_or(&self.dir)
            .join("remote-versions.json");

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

    #[instrument(skip_all)]
    pub fn save_remote_versions(&self, data: &LoadVersionsOutput) -> Result<(), ProtoLayoutError> {
        json::write_file(
            self.dir_original
                .as_ref()
                .unwrap_or(&self.dir)
                .join("remote-versions.json"),
            data,
            false,
        )?;

        Ok(())
    }
}
