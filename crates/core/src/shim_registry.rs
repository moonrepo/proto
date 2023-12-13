use crate::helpers::{read_json_file_with_lock, write_json_file_with_lock};
use crate::proto::ProtoEnvironment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Keep in sync with crates/cli-shim/src/main.rs
#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Shim {
    after_args: Vec<String>,
    alt_for: Option<String>,
    before_args: Vec<String>,
}

pub type ShimsMap = HashMap<String, Shim>;

pub struct ShimRegistry;

impl ShimRegistry {
    pub fn update<P: AsRef<ProtoEnvironment>, F: FnOnce(&mut ShimsMap)>(
        proto: P,
        op: F,
    ) -> miette::Result<()> {
        let file = proto.as_ref().shims_dir.join("registry.json");

        let mut config: ShimsMap = if file.exists() {
            read_json_file_with_lock(&file)?
        } else {
            HashMap::default()
        };

        op(&mut config);

        write_json_file_with_lock(file, &config)?;

        Ok(())
    }
}
