use proto_core::ProtoError;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const MANIFEST_NAME: &str = "manifest.json";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Manifest {
    pub default_version: String,
    pub installed_versions: FxHashSet<String>,
}

impl Manifest {
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

        let manifest: Manifest = serde_json::from_str(&contents)
            .map_err(|e| ProtoError::Json(path.to_path_buf(), e.to_string()))?;

        Ok(manifest)
    }

    pub fn save_to<P: AsRef<Path>>(&self, dir: P) -> Result<(), ProtoError> {
        self.save(dir.as_ref().join(MANIFEST_NAME))?;

        Ok(())
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ProtoError> {
        let path = path.as_ref();

        let data = serde_json::to_string_pretty(self)
            .map_err(|e| ProtoError::Json(path.to_path_buf(), e.to_string()))?;

        let handle_error = |e: std::io::Error| ProtoError::Fs(path.to_path_buf(), e.to_string());

        fs::create_dir_all(path.parent().unwrap()).map_err(handle_error)?;
        fs::write(path, data).map_err(handle_error)?;

        Ok(())
    }
}
