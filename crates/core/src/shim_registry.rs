use crate::helpers::{read_json_file_with_lock, write_json_file_with_lock};
use crate::proto::ProtoEnvironment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Keep in sync with crates/cli-shim/src/main.rs
#[derive(Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Shim {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after_args: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt_for: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before_args: Vec<String>,
}

pub type ShimsMap = HashMap<String, Shim>;

pub struct ShimRegistry;

impl ShimRegistry {
    pub fn update<P: AsRef<ProtoEnvironment>>(proto: P, entries: ShimsMap) -> miette::Result<()> {
        let file = proto.as_ref().shims_dir.join("registry.json");

        let mut config: ShimsMap = if file.exists() {
            read_json_file_with_lock(&file)?
        } else {
            HashMap::default()
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
