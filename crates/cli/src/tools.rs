use convert_case::{Case, Casing};
use proto_core::*;
use proto_node as node;
use proto_rust as rust;
use proto_schema_plugin as schema_plugin;
use proto_wasm_plugin as wasm_plugin;
use starbase_utils::toml;
use std::{env, path::Path, str::FromStr};
use strum::EnumIter;
use tracing::debug;

#[derive(Clone, Debug, Eq, EnumIter, Hash, PartialEq)]
pub enum ToolType {
    // Node.js
    Node,
    Npm,
    Pnpm,
    Yarn,
    // Rust
    Rust,
    // Plugins
    Plugin(String),
}

impl FromStr for ToolType {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            // Node.js
            "node" => Ok(Self::Node),
            "npm" => Ok(Self::Npm),
            "pnpm" => Ok(Self::Pnpm),
            "yarn" | "yarnpkg" => Ok(Self::Yarn),
            // Rust
            "rust" => Ok(Self::Rust),
            // Plugins
            name => Ok(Self::Plugin(name.to_case(Case::Kebab))),
        }
    }
}

pub async fn create_plugin_from_locator(
    plugin: &str,
    proto: impl AsRef<Proto>,
    locator: impl AsRef<PluginLocator>,
) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    match locator.as_ref() {
        PluginLocator::Source(location) => {
            let (is_toml, source_path) = match location {
                PluginLocation::File(file) => {
                    if !file.exists() {
                        return Err(ProtoError::PluginFileMissing(file.to_path_buf()));
                    }

                    (
                        file.extension().is_some_and(|ext| ext == "toml"),
                        file.to_path_buf(),
                    )
                }
                PluginLocation::Url(url) => {
                    (url.ends_with(".toml"), download_plugin(plugin, url).await?)
                }
            };

            if is_toml {
                debug!(source = ?source_path, "Loading TOML plugin");

                return Ok(Box::new(schema_plugin::SchemaPlugin::new(
                    proto,
                    plugin.to_owned(),
                    toml::read_file(source_path)?,
                )));
            }

            debug!(source = ?source_path, "Loading WASM plugin");

            Ok(Box::new(wasm_plugin::WasmPlugin::new(
                proto,
                plugin.to_owned(),
                source_path,
            )?))
        }
    }
}

pub async fn create_plugin_tool(
    plugin: &str,
    proto: Proto,
) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    let mut locator = None;

    // Traverse upwards checking each `.prototools` for a plugin
    if let Ok(working_dir) = env::current_dir() {
        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            let tools_config = ToolsConfig::load_from(dir)?;

            if let Some(maybe_locator) = tools_config.plugins.get(plugin) {
                locator = Some(maybe_locator.to_owned());
                break;
            }

            current_dir = dir.parent();
        }
    }

    // Otherwise fallback to the user's config
    if locator.is_none() {
        let user_config = UserConfig::load()?;

        if let Some(maybe_locator) = user_config.plugins.get(plugin) {
            locator = Some(maybe_locator.to_owned());
        }
    }

    let Some(locator) = locator else {
        return Err(ProtoError::MissingPlugin(plugin.to_owned()));
    };

    create_plugin_from_locator(plugin, proto, locator).await
}

pub async fn create_tool(tool: &ToolType) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    let proto = Proto::new()?;

    Ok(match tool {
        // Node.js
        ToolType::Node => Box::new(node::NodeLanguage::new(proto)),
        ToolType::Npm => Box::new(node::NodeDependencyManager::new(
            proto,
            node::NodeDependencyManagerType::Npm,
        )),
        ToolType::Pnpm => Box::new(node::NodeDependencyManager::new(
            proto,
            node::NodeDependencyManagerType::Pnpm,
        )),
        ToolType::Yarn => Box::new(node::NodeDependencyManager::new(
            proto,
            node::NodeDependencyManagerType::Yarn,
        )),
        // Rust
        ToolType::Rust => Box::new(rust::RustLanguage::new(proto)),
        // Plugins
        ToolType::Plugin(plugin) => create_plugin_tool(plugin, proto).await?,
    })
}
