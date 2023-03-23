use crate::errors::ProtoError;
use rustc_hash::FxHashMap;
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml::{map::Map, Value};

pub const CONFIG_NAME: &str = ".prototools";

#[derive(Debug, Default)]
pub struct ToolsConfig {
    pub tools: FxHashMap<String, String>,
    pub path: PathBuf,
}

impl ToolsConfig {
    pub fn load_upwards<P>(dir: P) -> Result<Option<Self>, ProtoError>
    where
        P: AsRef<Path>,
    {
        let dir = dir.as_ref();
        let findable = dir.join(CONFIG_NAME);

        if findable.exists() {
            return Ok(Some(Self::load(&findable)?));
        }

        match dir.parent() {
            Some(parent_dir) => Self::load_upwards(parent_dir),
            None => Ok(None),
        }
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoError> {
        Self::load(dir.as_ref().join(CONFIG_NAME))
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ProtoError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(ToolsConfig {
                path: path.to_owned(),
                ..ToolsConfig::default()
            });
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| ProtoError::Fs(path.to_path_buf(), e.to_string()))?;

        let config = contents
            .parse::<Value>()
            .map_err(|e| ProtoError::InvalidConfig(path.to_path_buf(), e.to_string()))?;

        let mut tools = FxHashMap::default();

        if let Value::Table(table) = config {
            for (key, value) in table {
                if let Value::String(version) = value {
                    tools.insert(key, version);
                } else {
                    return Err(ProtoError::InvalidConfig(
                        path.to_path_buf(),
                        format!("Expected a version string for \"{key}\"."),
                    ));
                }
            }
        } else {
            return Err(ProtoError::InvalidConfig(
                path.to_path_buf(),
                "Expected a mapping of tools to versions.".into(),
            ));
        }

        Ok(ToolsConfig {
            tools,
            path: path.to_owned(),
        })
    }

    pub fn save(&self) -> Result<(), ProtoError> {
        let mut map = Map::with_capacity(self.tools.len());

        for (tool, version) in &self.tools {
            map.insert(tool.to_owned(), Value::String(version.to_owned()));
        }

        let data = toml::to_string_pretty(&Value::Table(map))
            .map_err(|e| ProtoError::Toml(self.path.to_path_buf(), e.to_string()))?;

        fs::write(&self.path, data)
            .map_err(|e| ProtoError::Fs(self.path.to_path_buf(), e.to_string()))?;

        Ok(())
    }
}
