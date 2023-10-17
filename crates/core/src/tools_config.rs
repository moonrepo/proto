use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use starbase_utils::{fs, toml};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{debug, trace};
use version_spec::*;
use warpgate::{Id, PluginLocator};

pub const TOOLS_CONFIG_NAME: &str = ".prototools";

fn is_empty<T>(map: &BTreeMap<Id, T>) -> bool {
    map.is_empty()
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ToolsConfig {
    #[serde(flatten, skip_serializing_if = "is_empty")]
    pub tools: BTreeMap<Id, UnresolvedVersionSpec>,

    #[serde(skip_serializing_if = "is_empty")]
    pub plugins: BTreeMap<Id, PluginLocator>,

    #[serde(skip)]
    pub path: PathBuf,
}

impl ToolsConfig {
    pub fn builtin_plugins() -> BTreeMap<Id, PluginLocator> {
        let mut config = ToolsConfig::default();
        config.inherit_builtin_plugins();
        config.plugins
    }

    pub fn schema_plugin() -> PluginLocator {
        PluginLocator::SourceUrl {
            url: "https://github.com/moonrepo/schema-plugin/releases/latest/download/schema_plugin.wasm".into()
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn load() -> miette::Result<Self> {
        let working_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self::load_from(working_dir)
    }

    pub fn load_from<P: AsRef<Path>>(dir: P) -> miette::Result<Self> {
        let path = dir.as_ref().join(TOOLS_CONFIG_NAME);

        let mut config: ToolsConfig = if path.exists() {
            debug!(file = ?path, "Loading {}", TOOLS_CONFIG_NAME);

            toml::from_str(&fs::read_file(&path)?).into_diagnostic()?
        } else {
            ToolsConfig::default()
        };

        config.path = path.clone();

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

    pub fn load_closest() -> miette::Result<Self> {
        let working_dir = env::current_dir().expect("Unknown current working directory!");

        Self::load_upwards_from(working_dir, true)
    }

    pub fn load_upwards() -> miette::Result<Self> {
        let working_dir = env::current_dir().expect("Unknown current working directory!");

        Self::load_upwards_from(working_dir, false)
    }

    pub fn load_upwards_from<P>(starting_dir: P, stop_at_first: bool) -> miette::Result<Self>
    where
        P: AsRef<Path>,
    {
        trace!("Traversing upwards and loading .prototools files");

        let mut current_dir = Some(starting_dir.as_ref());
        let mut config = ToolsConfig::default();

        while let Some(dir) = current_dir {
            if dir.join(TOOLS_CONFIG_NAME).exists() {
                let mut parent_config = Self::load_from(dir)?;
                parent_config.merge(config);

                config = parent_config;

                if stop_at_first {
                    break;
                }
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
        fs::write_file(&self.path, toml::to_string_pretty(self).into_diagnostic()?)?;

        Ok(())
    }

    pub fn inherit_builtin_plugins(&mut self) {
        if !self.plugins.contains_key("bun") {
            self.plugins.insert(
                Id::raw("bun"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/bun-plugin/releases/latest/download/bun_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("deno") {
            self.plugins.insert(
                Id::raw("deno"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/deno-plugin/releases/latest/download/deno_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("go") {
            self.plugins.insert(
                Id::raw("go"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/go-plugin/releases/latest/download/go_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("node") {
            self.plugins.insert(
                Id::raw("node"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_plugin.wasm".into()
                }
            );
        }

        for depman in ["npm", "pnpm", "yarn"] {
            if !self.plugins.contains_key(depman) {
                self.plugins.insert(
                    Id::raw(depman),
                    PluginLocator::SourceUrl {
                        url: "https://github.com/moonrepo/node-plugin/releases/latest/download/node_depman_plugin.wasm".into()
                    }
                );
            }
        }

        if !self.plugins.contains_key("python") {
            self.plugins.insert(
                Id::raw("python"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/python-plugin/releases/latest/download/python_plugin.wasm".into()
                }
            );
        }

        if !self.plugins.contains_key("rust") {
            self.plugins.insert(
                Id::raw("rust"),
                PluginLocator::SourceUrl {
                    url: "https://github.com/moonrepo/rust-plugin/releases/latest/download/rust_plugin.wasm".into()
                }
            );
        }
    }

    pub fn merge(&mut self, other: ToolsConfig) {
        self.tools.extend(other.tools);
        self.plugins.extend(other.plugins);
    }
}
