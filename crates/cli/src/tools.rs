use proto_core::*;
use proto_wasm_plugin::Wasm;
use starbase_utils::fs;
use std::collections::HashMap;
use std::{env, path::Path};
use tracing::debug;

pub async fn create_tool_from_plugin(
    id: impl AsRef<Id>,
    proto: impl AsRef<ProtoEnvironment>,
    locator: impl AsRef<PluginLocator>,
) -> miette::Result<Tool> {
    let id = id.as_ref();
    let proto = proto.as_ref();
    let locator = locator.as_ref();
    let loader = PluginLoader::new(&proto.plugins_dir, &proto.temp_dir);

    let plugin_path = loader.load_plugin(&id, locator).await?;

    // If a TOML plugin, we need to load the WASM plugin for it,
    // wrap it, and modify the plugin manifest.
    if plugin_path
        .extension()
        .map(|ext| ext == "toml")
        .unwrap_or(false)
    {
        debug!(source = ?plugin_path, "Loading TOML plugin");

        let mut config = HashMap::new();
        config.insert("schema".to_string(), fs::read_file(plugin_path)?);

        let plugin_path = loader.load_plugin(id, ToolsConfig::schema_plugin()).await?;

        let mut manifest = Tool::create_plugin_manifest(proto, Wasm::file(plugin_path))?;
        manifest = manifest.with_config(config.into_iter());

        return Tool::load_from_manifest(id, proto, manifest);
    }

    // Otherwise, just use the WASM plugin as is
    debug!(source = ?plugin_path, "Loading WASM plugin");

    Tool::load(id, proto, Wasm::file(plugin_path))
}

pub async fn create_tool(id: &Id) -> miette::Result<Tool> {
    let proto = ProtoEnvironment::new()?;
    let mut locator = None;

    debug!(
        tool = id.as_str(),
        "Traversing upwards to find a configured plugin"
    );

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

    create_tool_from_plugin(id, proto, locator).await
}
