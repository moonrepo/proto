// use convert_case::{Case, Casing};
use proto_core::*;
// use proto_schema_plugin as schema_plugin;
use proto_wasm_plugin::Wasm;
// use starbase_utils::toml;
use std::{env, path::Path};
use tracing::debug;
use warpgate::{PluginLoader, PluginLocator};

pub async fn create_tool_from_plugin(
    id: &str,
    proto: impl AsRef<ProtoEnvironment>,
    locator: impl AsRef<PluginLocator>,
) -> miette::Result<Tool> {
    let proto = proto.as_ref();
    let locator = locator.as_ref();

    let plugin_path = PluginLoader::new(&proto.plugins_dir, &proto.temp_dir)
        .load_plugin(id, locator)
        .await?;
    // let is_toml = plugin_path
    //     .extension()
    //     .map(|ext| ext == "toml")
    //     .unwrap_or(false);

    // if is_toml {
    //     debug!(source = ?plugin_path, "Loading TOML plugin");

    //     return Ok(Box::new(schema_plugin::SchemaPlugin::new(
    //         proto,
    //         plugin.to_owned(),
    //         toml::read_file(plugin_path)?,
    //     )));
    // }

    debug!(source = ?plugin_path, "Loading WASM plugin");

    Ok(Tool::load(id, proto, Wasm::file(plugin_path))?)
}

pub async fn create_tool(id: &str) -> miette::Result<Tool> {
    let proto = ProtoEnvironment::new()?;
    let mut locator = None;

    debug!(tool = id, "Traversing upwards to find a configured plugin");

    // Traverse upwards checking each `.prototools` for a plugin
    if let Ok(working_dir) = env::current_dir() {
        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = &current_dir {
            let tools_config = ToolsConfig::load_from(dir)?;

            if let Some(maybe_locator) = tools_config.plugins.get(id) {
                locator = Some(maybe_locator.to_owned());
                break;
            }

            current_dir = dir.parent();
        }
    }

    // Then check the user's config
    if locator.is_none() {
        let user_config = UserConfig::load()?;

        if let Some(maybe_locator) = user_config.plugins.get(id) {
            locator = Some(maybe_locator.to_owned());
        }
    }

    // And finally the builtin plugins
    if locator.is_none() {
        let builtin_plugins = ToolsConfig::builtin_plugins();

        if let Some(maybe_locator) = builtin_plugins.get(id) {
            locator = Some(maybe_locator.to_owned());
        }
    }

    let Some(locator) = locator else {
        return Err(ProtoError::UnknownTool { id: id.to_owned() }.into());
    };

    Ok(create_tool_from_plugin(id, proto, locator).await?)
}
