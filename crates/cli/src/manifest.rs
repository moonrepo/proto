use proto_core::{get_tools_dir, ProtoError, Tool};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const MANIFEST_NAME: &str = "manifest.json";

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Manifest {
    pub default_version: String,
    pub installed_versions: FxHashSet<String>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl Manifest {
    pub fn load_for_tool(tool: &Box<dyn Tool<'_>>) -> Result<Self, ProtoError> {
        Self::load_from(get_tools_dir()?.join(tool.get_bin_name()))
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoError> {
        Self::load(dir.as_ref().join(MANIFEST_NAME))
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ProtoError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Manifest::default());
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?;

        let mut manifest: Manifest = serde_json::from_str(&contents)
            .map_err(|e| ProtoError::Json(path.to_path_buf(), e.to_string()))?;

        manifest.path = path.to_owned();

        Ok(manifest)
    }

    pub fn save(&self) -> Result<(), ProtoError> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| ProtoError::Json(self.path.to_path_buf(), e.to_string()))?;

        let handle_error =
            |e: std::io::Error| ProtoError::Fs(self.path.to_path_buf(), e.to_string());

        fs::create_dir_all(self.path.parent().unwrap()).map_err(handle_error)?;
        fs::write(&self.path, data).map_err(handle_error)?;

        Ok(())
    }
}
