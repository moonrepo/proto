mod detect;
mod download;
mod execute;
mod install;
mod resolve;
mod schema;
mod shim;
mod verify;

use proto_core::{Describable, Proto, ProtoError, Resolvable, Tool};
pub use schema::*;
use std::{
    env::consts,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct SchemaPlugin {
    pub schema: Schema,
    pub id: String,
    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl SchemaPlugin {
    pub fn new<P: AsRef<Proto>>(proto: P, id: String, schema: Schema) -> Self {
        let proto = proto.as_ref();

        SchemaPlugin {
            base_dir: proto.tools_dir.join(&id),
            bin_path: None,
            shim_path: None,
            temp_dir: proto.temp_dir.join(&id),
            version: None,
            id,
            schema,
        }
    }

    pub fn get_platform(&self) -> Result<&PlatformMapper, ProtoError> {
        let mut platform = self.schema.platform.get(consts::OS);

        // Fallback to linux for other OSes
        if platform.is_none() && consts::OS.ends_with("bsd") {
            platform = self.schema.platform.get("linux");
        }

        platform
            .ok_or_else(|| ProtoError::UnsupportedPlatform(self.get_name(), consts::OS.to_owned()))
    }

    pub fn get_checksum_file(&self) -> Result<String, ProtoError> {
        Ok(if let Some(file) = &self.get_platform()?.checksum_file {
            self.interpolate_tokens(file)
        } else {
            format!("v{}-SHASUMS256.txt", self.get_resolved_version())
        })
    }

    pub fn get_download_file(&self) -> Result<String, ProtoError> {
        Ok(self.interpolate_tokens(&self.get_platform()?.download_file))
    }

    pub fn interpolate_tokens(&self, value: &str) -> String {
        let mut value = value
            .replace("{version}", self.get_resolved_version())
            .replace("{arch}", self.schema.get_arch());

        // Avoid detecting musl unless requested
        if value.contains("{libc}") {
            value = value.replace("{libc}", self.schema.get_libc());
        }

        value
    }
}

impl Describable<'_> for SchemaPlugin {
    fn get_bin_name(&self) -> &str {
        &self.id
    }

    fn get_name(&self) -> String {
        self.schema.name.clone()
    }
}

impl Tool<'_> for SchemaPlugin {
    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
