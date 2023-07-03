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

pub struct PythonLanguage {
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub rye_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub version: Option<String>,

    manifest: OnceCell<Manifest>,
}

impl PythonLanguage {
    pub fn new<P: AsRef<Proto>>(proto: P) -> Self {
        let proto = proto.as_ref();

        PythonLanguage {
            base_dir: proto.tools_dir.join("python"),
            bin_path: None,
            manifest: OnceCell::new(),
            rye_dir: proto.home_dir.join(".rye"),
            temp_dir: proto.temp_dir.join("python"),
            version: None,
        }
    }
}

impl Describable<'_> for PythonLanguage {
    // This is actually an ID, not the actual bin name... revisit!
    fn get_id(&self) -> &str {
        "python"
    }

    fn get_name(&self) -> String {
        "Python".into()
    }
}

impl_tool!(PythonLanguage);
