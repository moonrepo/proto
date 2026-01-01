use super::layout_error::ProtoLayoutError;
use crate::helpers::{read_json_file_with_lock, write_json_file_with_lock};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Shim {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after_args: Vec<String>,

    // TODO rename to alt_exe in next version
    #[serde(skip_serializing_if = "Option::is_none", alias = "alt_exe")]
    pub alt_bin: Option<bool>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before_args: Vec<String>,

    // TODO rename to context in next version
    #[serde(skip_serializing_if = "Option::is_none", alias = "context")]
    pub parent: Option<String>,

    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub env_vars: FxHashMap<String, String>,
}

pub type ShimsMap = BTreeMap<String, Shim>;

pub struct ShimRegistry;

impl ShimRegistry {
    pub fn update(shims_dir: &Path, entries: ShimsMap) -> Result<(), ProtoLayoutError> {
        if entries.is_empty() {
            return Ok(());
        }

        let file = shims_dir.join("registry.json");

        let mut config: ShimsMap = if file.exists() {
            read_json_file_with_lock(&file)?
        } else {
            BTreeMap::default()
        };

        let mut mutated = false;

        for (key, value) in entries {
            // Don't write the file if nothing has changed
            if config
                .get(&key)
                .is_some_and(|current_value| current_value == &value)
            {
                continue;
            }

            config.insert(key, value);
            mutated = true;
        }

        if mutated {
            write_json_file_with_lock(file, &config)?;
        }

        Ok(())
    }
}
