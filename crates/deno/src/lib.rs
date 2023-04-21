mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
mod resolve;
mod shim;
mod verify;

use proto_core::{Describable, Proto, Tool};
use std::{
    any::Any,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct DenoLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl DenoLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        DenoLanguage {
            base_dir: proto.tools_dir.join("deno"),
            bin_path: None,
            temp_dir: proto.temp_dir.join("deno"),
            version: None,
        }
    }
}

impl Describable<'_> for DenoLanguage {
    fn get_bin_name(&self) -> &str {
        "deno"
    }

    fn get_name(&self) -> String {
        "Deno".into()
    }
}

impl Tool<'_> for DenoLanguage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
