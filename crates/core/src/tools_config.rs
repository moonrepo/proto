use crate::errors::ProtoError;
use rustc_hash::FxHashMap;
use starbase_utils::toml::{self, value::Table, value::Value};
use std::path::{Path, PathBuf};

pub const TOOLS_CONFIG_NAME: &str = ".prototools";

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
        let findable = dir.join(TOOLS_CONFIG_NAME);

        if findable.exists() {
            return Ok(Some(Self::load(&findable)?));
        }

        match dir.parent() {
            Some(parent_dir) => Self::load_upwards(parent_dir),
            None => Ok(None),
        }
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoError> {
        Self::load(dir.as_ref().join(TOOLS_CONFIG_NAME))
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ProtoError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(ToolsConfig {
                path: path.to_owned(),
                ..ToolsConfig::default()
            });
        }

        let config: Value = toml::read_file(path)?;
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
        let mut map = Table::with_capacity(self.tools.len());

        for (tool, version) in &self.tools {
            map.insert(tool.to_owned(), Value::String(version.to_owned()));
        }

        toml::write_file(&self.path, &Value::Table(map), true)?;

        Ok(())
    }
}
