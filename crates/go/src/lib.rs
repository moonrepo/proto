mod detect;
pub mod download;
mod execute;
mod install;
mod platform;
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
pub struct GoLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    manifest: OnceCell<Manifest>,
}

impl GoLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        GoLanguage {
            base_dir: proto.tools_dir.join("go"),
            bin_path: None,
            manifest: OnceCell::new(),
            temp_dir: proto.temp_dir.join("go"),
            version: None,
        }
    }
}

impl Describable<'_> for GoLanguage {
    fn get_id(&self) -> &str {
        "go"
    }

    fn get_name(&self) -> String {
        "Go".into()
    }
}

impl_tool!(GoLanguage);
