use convert_case::{Case, Casing};
use proto_bun as bun;
use proto_core::*;
use proto_deno as deno;
use proto_go as go;
use proto_node as node;
use proto_rust as rust;
use proto_schema_plugin as schema_plugin;
use starbase_utils::toml;
use std::{
    env,
    path::{Path, PathBuf},
    str::FromStr,
};
use strum::EnumIter;

#[derive(Clone, Debug, Eq, EnumIter, Hash, PartialEq)]
pub enum ToolType {
    // Bun
    Bun,
    // Deno
    Deno,
    // Go
    Go,
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
            // Bun
            "bun" => Ok(Self::Bun),
            // Deno
            "deno" => Ok(Self::Deno),
            // Go
            "go" => Ok(Self::Go),
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
    proto: Proto,
    locator: PluginLocator,
    source_dir: PathBuf,
) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    match locator {
        PluginLocator::Schema(location) => {
            let schema: schema_plugin::Schema = match location {
                PluginLocation::File(file) => {
                    let file_path = source_dir.join(file);

                    if !file_path.exists() {
                        return Err(ProtoError::PluginFileMissing(file_path));
                    }

                    toml::read_file(file_path)?
                }
                PluginLocation::Url(url) => toml::read_file(download_plugin(plugin, url).await?)?,
            };

            Ok(Box::new(schema_plugin::SchemaPlugin::new(proto, schema)))
        }
    }
}

pub async fn create_plugin_tool(
    plugin: &str,
    proto: Proto,
) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    let mut locator = None;
    let mut parent_dir = PathBuf::new();

    // Traverse upwards checking each `.prototools` for a plugin
    if let Ok(working_dir) = env::current_dir() {
        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            let tools_config = ToolsConfig::load_from(dir)?;

            if let Some(maybe_locator) = tools_config.plugins.get(plugin) {
                locator = Some(maybe_locator.to_owned());
                parent_dir = dir.to_path_buf();
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
            parent_dir = get_root()?;
        }
    }

    let Some(locator) = locator else {
        return Err(ProtoError::MissingPlugin(plugin.to_owned()));
    };

    create_plugin_from_locator(plugin, proto, locator, parent_dir).await
}

pub async fn create_tool(tool: &ToolType) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    let proto = Proto::new()?;

    Ok(match tool {
        // Bun
        ToolType::Bun => Box::new(bun::BunLanguage::new(proto)),
        // Deno
        ToolType::Deno => Box::new(deno::DenoLanguage::new(proto)),
        // Go
        ToolType::Go => Box::new(go::GoLanguage::new(proto)),
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
