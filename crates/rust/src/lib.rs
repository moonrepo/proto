mod detect;
pub mod download;
mod execute;
mod install;
mod resolve;
mod shim;
mod verify;

use proto_core::{Describable, Proto, Tool};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct RustLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl RustLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        RustLanguage {
            base_dir: proto.home_dir.join(".rustup").join("toolchains"),
            bin_path: None,
            temp_dir: proto.temp_dir.join("rust"),
            version: None,
        }
    }
}

impl Describable<'_> for RustLanguage {
    fn get_bin_name(&self) -> &str {
        "rust"
    }

    fn get_name(&self) -> String {
        "Rust".into()
    }
}

impl Tool<'_> for RustLanguage {
    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
