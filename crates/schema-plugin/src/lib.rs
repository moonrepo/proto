mod detect;
mod download;
mod execute;
mod install;
// mod resolve;
mod schema;
mod shim;
mod verify;

use proto_core::{Describable, Proto, Tool};
pub use schema::*;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SchemaPlugin {
    pub schema: ToolSchema,

    pub base_dir: PathBuf,
    pub bin_path: Option<PathBuf>,
    pub shim_path: Option<PathBuf>,
    pub temp_dir: PathBuf,
    pub version: Option<String>,
}

impl SchemaPlugin {
    pub fn new<P: AsRef<Proto>>(proto: P, schema: ToolSchema) -> Self {
        let proto = proto.as_ref();

        SchemaPlugin {
            base_dir: proto.tools_dir.join(&schema.id),
            bin_path: None,
            shim_path: None,
            temp_dir: proto.temp_dir.join(&schema.id),
            version: None,
            schema,
        }
    }

    pub fn format_string(&self, value: &str) -> String {
        value
            .replace("{version}", self.get_resolved_version())
            .replace("{arch}", self.schema.get_arch())
            .replace("{os}", self.schema.get_os())
            .replace("{ext}", self.schema.get_download_ext())
    }
}

impl Describable<'_> for SchemaPlugin {
    fn get_bin_name(&self) -> &str {
        &self.schema.id
    }

    fn get_name(&self) -> String {
        self.schema.name.into()
    }
}

impl Tool<'_> for SchemaPlugin {
    fn get_tool_dir(&self) -> &Path {
        &self.base_dir
    }
}
