mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
mod resolve;
mod shim;
mod verify;

use proto_core::{Describable, Proto, Tool};
use std::path::PathBuf;

#[derive(Debug)]
pub struct BunLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub log_target: String,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl BunLanguage {
    pub fn new(proto: &Proto) -> Self {
        BunLanguage {
            base_dir: proto.tools_dir.join("bun"),
            bin_path: None,
            log_target: "proto:tool:bun".into(),
            temp_dir: proto.temp_dir.join("bun"),
            version: None,
        }
    }
}

impl Describable<'_> for BunLanguage {
    fn get_bin_name(&self) -> &str {
        "bun"
    }

    fn get_log_target(&self) -> &str {
        &self.log_target
    }

    fn get_name(&self) -> String {
        "Bun".into()
    }
}

impl Tool<'_> for BunLanguage {}
