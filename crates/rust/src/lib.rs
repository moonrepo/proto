mod detect;
pub mod download;
mod execute;
mod install;
mod resolve;
mod shim;
mod verify;

use once_cell::sync::OnceCell;
use proto_core::{impl_tool, is_musl, Describable, Manifest, Proto, ProtoError, Tool};
use std::{
    any::Any,
    env::consts,
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

pub fn get_triple_target() -> Result<String, ProtoError> {
    Ok(match consts::OS {
        "linux" => format!(
            "{}-unknown-linux-{}",
            consts::ARCH,
            if is_musl() { "musl" } else { "gnu" }
        ),
        "macos" => format!("{}-apple-darwin", consts::ARCH),
        "windows" => format!("{}-pc-windows-msvc", consts::ARCH),
        other => return Err(ProtoError::UnsupportedPlatform("Rust".into(), other.into())),
    })
}
