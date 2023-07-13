use crate::{errors::ProtoError, plugin::PluginLocator};
use convert_case::{Case, Casing};
use rustc_hash::FxHashMap;
use starbase_utils::toml::{self, TomlTable, TomlValue};
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const TOOLS_CONFIG_NAME: &str = ".prototools";

#[derive(Debug, Default)]
pub struct ToolsConfig {
    pub tools: FxHashMap<String, String>,
    pub plugins: FxHashMap<String, PluginLocator>,
    pub path: PathBuf,
}

impl ToolsConfig {
    pub fn load_upwards() -> Result<Self, ProtoError> {
        let working_dir = env::current_dir().expect("Unknown current working directory!");
        let mut current_dir = Some(working_dir.as_path());
        let mut config = ToolsConfig::default();

        while let Some(dir) = current_dir {
            let path = dir.join(TOOLS_CONFIG_NAME);

            if path.exists() {
                let mut parent_config = Self::load(&path)?;
                parent_config.merge(config);

                config = parent_config;
            }

            match dir.parent() {
                Some(parent) => {
                    current_dir = Some(parent);
                }
                None => {
                    break;
                }
            };
        }

        Ok(config)
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> Result<Self, ProtoError> {
        Self::load(dir.as_ref().join(TOOLS_CONFIG_NAME))
    }

    #[tracing::instrument(skip_all)]
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ProtoError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(ToolsConfig {
                path: path.to_owned(),
                ..ToolsConfig::default()
            });
        }

        let config: TomlValue = toml::read_file(path)?;
        let mut tools = FxHashMap::default();
        let mut plugins = FxHashMap::default();

        if let TomlValue::Table(table) = config {
            for (key, value) in table {
                match value {
                    TomlValue::String(version) => {
                        tools.insert(key, version);
                    }
                    TomlValue::Table(plugins_table) => {
                        for (plugin, locator) in plugins_table {
                            if let TomlValue::String(locator) = locator {
                                plugins.insert(
                                    plugin.to_case(Case::Kebab),
                                    PluginLocator::from_str(&locator)?,
                                );
                            } else {
                                return Err(ProtoError::InvalidConfig(
                                    path.to_path_buf(),
                                    format!(
                                        "Invalid plugin \"{plugin}\", expected a locator string."
                                    ),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(ProtoError::InvalidConfig(
                            path.to_path_buf(),
                            format!(
                                "Invalid field \"{key}\", expected a mapped tool version, or a [plugins] map."
                            ),
                        ))
                    }
                }
            }
        } else {
            return Err(ProtoError::InvalidConfig(
                path.to_path_buf(),
                "Expected a mapping of tools or plugins.".into(),
            ));
        }

        Ok(ToolsConfig {
            tools,
            plugins,
            path: path.to_owned(),
        })
    }

    pub fn merge(&mut self, other: ToolsConfig) {
        self.tools.extend(other.tools);
        self.plugins.extend(other.plugins);
    }

    #[tracing::instrument(skip_all)]
    pub fn save(&self) -> Result<(), ProtoError> {
        let mut map = TomlTable::with_capacity(self.tools.len());

        for (tool, version) in &self.tools {
            map.insert(tool.to_owned(), TomlValue::String(version.to_owned()));
        }

        if !self.plugins.is_empty() {
            let mut plugins = TomlTable::with_capacity(self.plugins.len());

            for (plugin, locator) in &self.plugins {
                plugins.insert(plugin.to_owned(), TomlValue::String(locator.to_string()));
            }

            map.insert("plugins".to_owned(), TomlValue::Table(plugins));
        }

        toml::write_file(&self.path, &TomlValue::Table(map), true)?;

        Ok(())
    }
}
