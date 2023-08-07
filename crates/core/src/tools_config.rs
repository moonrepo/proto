use crate::version::AliasOrVersion;
use serde::{Deserialize, Serialize};
use starbase_utils::toml;
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};
use warpgate::PluginLocator;

pub const TOOLS_CONFIG_NAME: &str = ".prototools";

fn is_empty<T>(map: &BTreeMap<String, T>) -> bool {
    map.is_empty()
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolsConfig {
    #[serde(flatten, skip_serializing_if = "is_empty")]
    pub tools: BTreeMap<String, AliasOrVersion>,

    #[serde(skip_serializing_if = "is_empty")]
    pub plugins: BTreeMap<String, PluginLocator>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl ToolsConfig {
    pub fn builtin_plugins() -> BTreeMap<String, PluginLocator> {
        let mut config = ToolsConfig::default();
        config.inherit_builtin_plugins();
        config.plugins
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        Self::load(dir.as_ref().join(TOOLS_CONFIG_NAME))
    }

    #[tracing::instrument(skip_all)]
    pub fn load<P: AsRef<Path>>(path: P) -> miette::Result<Self> {
        let path = path.as_ref();

        let mut config: ToolsConfig = if path.exists() {
            debug!(file = ?path, "Loading {}", TOOLS_CONFIG_NAME);

            toml::read_file(path)?
        } else {
            ToolsConfig::default()
        };

        config.path = path.to_owned();

        // Update plugin file paths to be absolute
        for locator in config.plugins.values_mut() {
            if let PluginLocator::SourceFile {
                path: ref mut source_path,
                ..
            } = locator
            {
                *source_path = path.parent().unwrap().join(&source_path);
            }
        }

        Ok(config)
    }

    pub fn load_upwards() -> miette::Result<Self> {
        let working_dir = env::current_dir().expect("Unknown current working directory!");

        Self::load_upwards_from(working_dir)
    }

    pub fn load_upwards_from<P>(starting_dir: P) -> miette::Result<Self>
    where
        P: AsRef<Path>,
    {
        trace!("Traversing upwards and loading all .prototools files");

        let mut current_dir = Some(starting_dir.as_ref());
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

    pub fn save(&self) -> miette::Result<()> {
        toml::write_file(&self.path, self, true)?;

        Ok(())
    }

    pub fn inherit_builtin_plugins(&mut self) {
        if !self.plugins.contains_key("bun") {
            self.plugins.insert(
                "bun".into(),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/bun-plugin/releases/latest/download/bun_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("deno") {
            self.plugins.insert(
                "deno".into(),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/deno-plugin/releases/latest/download/deno_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("go") {
            self.plugins.insert(
                "go".into(),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/go-plugin/releases/latest/download/go_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("node") {
            self.plugins.insert(
                "node".into(),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_plugin.wasm".into()
                }
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !self.plugins.contains_key(depman) {
                self.plugins.insert(
                    depman.into(),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_depman_plugin.wasm".into()
                    }
                );
            }
        }
    }

    pub fn merge(&mut self, other: ToolsConfig) {
        self.tools.extend(other.tools);
        self.plugins.extend(other.plugins);
    }
}
