use miette::IntoDiagnostic;
use proto_core::*;
use proto_pdk_api::UserConfigSettings;
use proto_wasm_plugin::Wasm;
use starbase_utils::{fs, json};
use std::collections::HashMap;
use std::{env, path::Path};
use tracing::debug;

pub async fn create_tool_from_plugin(
    id: impl AsRef<Id>,
    proto: impl AsRef<ProtoEnvironment>,
    locator: impl AsRef<PluginLocator>,
    user_config: &UserConfig,
) -> miette::Result<Tool> {
    let id = id.as_ref();
    let proto = proto.as_ref();
    let locator = locator.as_ref();

    let mut loader = PluginLoader::new(&proto.plugins_dir, &proto.temp_dir);
    loader.set_seed(env!("CARGO_PKG_VERSION"));

    let plugin_path = loader.load_plugin(&id, locator).await?;
    let mut config = HashMap::new();

    // If a TOML plugin, we need to load the WASM plugin for it,
    // wrap it, and modify the plugin manifest.
    let mut manifest = if plugin_path
        .extension()
        .map(|ext| ext == "toml")
        .unwrap_or(false)
    {
        debug!(source = ?plugin_path, "Loading TOML plugin");

        config.insert("schema".to_string(), fs::read_file(plugin_path)?);

        Tool::create_plugin_manifest(
            proto,
            Wasm::file(loader.load_plugin(id, ToolsConfig::schema_plugin()).await?),
        )?

        // Otherwise, just use the WASM plugin as is
    } else {
        debug!(source = ?plugin_path, "Loading WASM plugin");

        Tool::create_plugin_manifest(proto, Wasm::file(plugin_path))?
    };

    config.insert("proto_tool_id".to_string(), id.to_string());

    config.insert(
        "proto_user_config".to_string(),
        json::to_string(&UserConfigSettings {
            auto_clean: user_config.auto_clean,
            auto_install: user_config.auto_install,
            node_intercept_globals: user_config.node_intercept_globals,
        })
        .into_diagnostic()?,
    );

    manifest.config.extend(config);

    Tool::load_from_manifest(id, proto, manifest)
}

pub async fn create_tool(id: &Id) -> miette::Result<Tool> {
    let proto = ProtoEnvironment::new()?;
    let user_config = UserConfig::load()?;
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

    create_tool_from_plugin(id, proto, locator, &user_config).await
}
