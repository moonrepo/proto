use crate::version::AliasOrVersion;
use serde::{Deserialize, Serialize};
use starbase_utils::toml;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::debug;
use warpgate::PluginLocator;

pub const TOOLS_CONFIG_NAME: &str = ".prototools";

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolsConfig {
    #[serde(flatten)]
    pub tools: BTreeMap<String, AliasOrVersion>,

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

        debug!(file = ?path, "Loading {}", TOOLS_CONFIG_NAME);

        let mut config: ToolsConfig = if path.exists() {
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
