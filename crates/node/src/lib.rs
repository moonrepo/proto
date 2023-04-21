pub mod depman;
mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
mod resolve;
mod shim;
mod verify;

pub use depman::*;
use proto_core::{Describable, Proto, Tool};
use std::{
    any::Any,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct NodeLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl NodeLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        NodeLanguage {
            base_dir: proto.tools_dir.join("node"),
            bin_path: None,
            shim_path: None,
            temp_dir: proto.temp_dir.join("node"),
            version: None,
        }
    }
}

impl Describable<'_> for NodeLanguage {
    fn get_bin_name(&self) -> &str {
        "node"
    }

    fn get_name(&self) -> String {
        "Node.js".into()
    }
}

impl Tool<'_> for NodeLanguage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
