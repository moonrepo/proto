mod detect;
pub mod download;
mod execute;
mod install;
mod resolve;
mod shim;
mod verify;

use once_cell::sync::OnceCell;
use proto_core::{impl_tool, Describable, Manifest, Proto, ProtoError, Tool};
use std::{
    any::Any,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct RustLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub rustup_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    manifest: OnceCell<Manifest>,
}

impl RustLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        RustLanguage {
            base_dir: proto.tools_dir.join("rust"),
            bin_path: None,
            manifest: OnceCell::new(),
            rustup_dir: proto.home_dir.join(".rustup").join("toolchains"),
            temp_dir: proto.temp_dir.join("rust"),
            version: None,
        }
    }
}

impl Describable<'_> for RustLanguage {
    // This is actually an ID, not the actual bin name... revisit!
    fn get_id(&self) -> &str {
        "rust"
    }

    fn get_name(&self) -> String {
        "Rust".into()
    }
}

impl_tool!(RustLanguage);
