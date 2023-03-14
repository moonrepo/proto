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
pub struct DenoLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub log_target: String,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl DenoLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        DenoLanguage {
            base_dir: proto.tools_dir.join("deno"),
            bin_path: None,
            log_target: "proto:tool:deno".into(),
            temp_dir: proto.temp_dir.join("deno"),
            version: None,
        }
    }
}

impl Describable<'_> for DenoLanguage {
    fn get_bin_name(&self) -> &str {
        "deno"
    }

    fn get_log_target(&self) -> &str {
        &self.log_target
    }

    fn get_name(&self) -> String {
        "Deno".into()
    }
}

impl Tool<'_> for DenoLanguage {}
