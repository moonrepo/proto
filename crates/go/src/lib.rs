mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
mod resolve;
mod shim;
mod verify;

use proto_core::{Describable, Proto, Tool};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct GoLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl GoLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        GoLanguage {
            base_dir: proto.tools_dir.join("go"),
            bin_path: None,
            temp_dir: proto.temp_dir.join("go"),
            version: None,
        }
    }
}

impl Describable<'_> for GoLanguage {
    fn get_bin_name(&self) -> &str {
        "go"
    }

    fn get_name(&self) -> String {
        "Go".into()
    }
}

impl Tool<'_> for GoLanguage {
    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
