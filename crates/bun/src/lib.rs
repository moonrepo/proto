mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
mod resolve;
mod shim;
mod verify;

use once_cell::sync::OnceCell;
use proto_core::{Describable, Manifest, Proto, ProtoError, Tool};
use std::{
    any::Any,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct BunLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    manifest: OnceCell<Manifest>,
}

impl BunLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        BunLanguage {
            base_dir: proto.tools_dir.join("bun"),
            bin_path: None,
            manifest: OnceCell::new(),
            temp_dir: proto.temp_dir.join("bun"),
            version: None,
        }
    }
}

impl Describable<'_> for BunLanguage {
    fn get_bin_name(&self) -> &str {
        "bun"
    }

    fn get_name(&self) -> String {
        "Bun".into()
    }
}

impl Tool<'_> for BunLanguage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_manifest(&self) -> Result<&Manifest, ProtoError> {
        self.manifest
            .get_or_try_init(|| Manifest::load(self.get_manifest_path()))
    }

    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
