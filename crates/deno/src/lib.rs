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
pub struct DenoLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    manifest: OnceCell<Manifest>,
}

impl DenoLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        DenoLanguage {
            base_dir: proto.tools_dir.join("deno"),
            bin_path: None,
            manifest: OnceCell::new(),
            temp_dir: proto.temp_dir.join("deno"),
            version: None,
        }
    }
}

impl Describable<'_> for DenoLanguage {
    fn get_id(&self) -> &str {
        "deno"
    }

    fn get_name(&self) -> String {
        "Deno".into()
    }
}

impl_tool!(DenoLanguage);
