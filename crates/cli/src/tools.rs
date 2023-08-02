use convert_case::{Case, Casing};
use proto_core::*;
use proto_schema_plugin as schema_plugin;
use proto_wasm_plugin as wasm_plugin;
use starbase_utils::toml;
use std::{env, path::Path, str::FromStr};
use strum::EnumIter;
use tracing::debug;
use warpgate::{PluginLoader, PluginLocator};

#[derive(Clone, Debug, Eq, EnumIter, Hash, PartialEq)]
pub enum ToolType {
    // Plugins
    Plugin(String),
}

impl ToolType {
    pub fn is(&self, id: &str) -> bool {
        match self {
            Self::Plugin(name) => name == id,
        }
    }
}

impl FromStr for ToolType {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::Plugin(value.to_lowercase().to_case(Case::Kebab)))
    }
}

pub async fn create_plugin_from_locator(
    plugin: &str,
    proto: impl AsRef<Proto>,
    locator: impl AsRef<PluginLocator>,
) -> Result<Box<dyn Tool<'static>>, ProtoError> {
    let proto = proto.as_ref();
    let locator = locator.as_ref();

    let plugin_path = PluginLoader::new(&proto.plugins_dir, &proto.temp_dir)
        .load_plugin(plugin, locator)
        .await
        .map_err(|e| ProtoError::Message(e.to_string()))?;
    let is_toml = plugin_path
        .extension()
        .map(|e| e == "toml")
        .unwrap_or(false);

    if is_toml {
        debug!(source = ?plugin_path, "Loading TOML plugin");

        return Ok(Box::new(schema_plugin::SchemaPlugin::new(
            proto,
            plugin.to_owned(),
            toml::read_file(plugin_path)?,
        )));
    }

    debug!(source = ?plugin_path, "Loading WASM plugin");

    Ok(Box::new(wasm_plugin::WasmPlugin::new(
        proto,
        plugin.to_owned(),
        plugin_path,
    )?))
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

    // Then check the user's config
    if locator.is_none() {
        let user_config = UserConfig::load()?;

        if let Some(maybe_locator) = user_config.plugins.get(plugin) {
            locator = Some(maybe_locator.to_owned());
        }
    }

    // And finally the builtin plugins
    if locator.is_none() {
        let builtin_plugins = ToolsConfig::builtin_plugins();

        if let Some(maybe_locator) = builtin_plugins.get(plugin) {
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
        // Plugins
        ToolType::Plugin(plugin) => create_plugin_tool(plugin, proto).await?,
    })
}
