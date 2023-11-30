use crate::error::ProtoError;
use crate::proto::ProtoEnvironment;
use crate::proto_config::{ProtoConfig, SCHEMA_PLUGIN_KEY};
use crate::tool::Tool;
use extism::{manifest::Wasm, Manifest};
use miette::IntoDiagnostic;
use proto_pdk_api::{HostArch, HostEnvironment, HostOS};
use starbase_utils::{json, toml};
use std::path::PathBuf;
use std::{env, path::Path};
use tracing::{debug, trace};
use warpgate::{to_virtual_path, Id, PluginLocator};

pub fn inject_default_manifest_config(
    id: &Id,
    proto: &ProtoEnvironment,
    manifest: &mut Manifest,
) -> miette::Result<()> {
    trace!(id = id.as_str(), "Storing tool identifier");

    manifest
        .config
        .insert("proto_tool_id".to_string(), id.to_string());

    let paths_map = manifest.allowed_paths.as_ref().unwrap();

    let value = json::to_string(&HostEnvironment {
        arch: HostArch::from_env(),
        os: HostOS::from_env(),
        home_dir: to_virtual_path(paths_map, &proto.home),
        proto_dir: to_virtual_path(paths_map, &proto.root),
    })
    .into_diagnostic()?;

    trace!(env = %value, "Storing proto environment");

    manifest
        .config
        .insert("proto_environment".to_string(), value);

    Ok(())
}

pub fn locate_tool(
    id: &Id,
    proto: &ProtoEnvironment,
    current_dir_only: bool,
) -> miette::Result<PluginLocator> {
    let mut locator = None;
    let configs = proto.load_config_manager()?;

    debug!(tool = id.as_str(), "Finding a configured plugin");

    // Check config files for plugins
    for (file, config) in configs.files.iter().rev() {
        if let Some(plugins) = &config.plugins {
            if let Some(maybe_locator) = plugins.get(id) {
                debug!(file = ?file, plugin = maybe_locator.to_string(), "Found a plugin");

                locator = Some(maybe_locator.to_owned());
                break;
            }
        }

        if current_dir_only {
            break;
        }
    }

    // And finally the built-in plugins
    if locator.is_none() {
        let builtin_plugins = ProtoConfig::builtin_plugins();

        if let Some(maybe_locator) = builtin_plugins.get(id) {
            debug!(
                plugin = maybe_locator.to_string(),
                "Using a built-in plugin"
            );

            locator = Some(maybe_locator.to_owned());
        }
    }

    let Some(locator) = locator else {
        return Err(ProtoError::UnknownTool { id: id.to_owned() }.into());
    };

    Ok(locator)
}

pub async fn load_schema_plugin(proto: impl AsRef<ProtoEnvironment>) -> miette::Result<PathBuf> {
    let proto = proto.as_ref();
    let http_client = proto.get_http_client()?;
    let plugin_loader = proto.get_plugin_loader();

    let schema_id = Id::raw(SCHEMA_PLUGIN_KEY);
    let schema_locator = locate_tool(&schema_id, proto, true)?;

    plugin_loader
        .load_plugin_with_client(schema_id, schema_locator, &http_client)
        .await
}

pub async fn load_tool_from_locator(
    id: impl AsRef<Id>,
    proto: impl AsRef<ProtoEnvironment>,
    locator: impl AsRef<PluginLocator>,
) -> miette::Result<Tool> {
    let id = id.as_ref();
    let proto = proto.as_ref();
    let locator = locator.as_ref();

    let http_client = proto.get_http_client()?;
    let plugin_loader = proto.get_plugin_loader();
    let plugin_path = plugin_loader
        .load_plugin_with_client(id, locator, http_client)
        .await?;

    // If a TOML plugin, we need to load the WASM plugin for it,
    // wrap it, and modify the plugin manifest.
    let mut manifest = if plugin_path
        .extension()
        .map(|ext| ext == "toml")
        .unwrap_or(false)
    {
        debug!(source = ?plugin_path, "Loading TOML plugin");

        let mut manifest =
            Tool::create_plugin_manifest(proto, Wasm::file(load_schema_plugin(proto).await?))?;

        // Convert TOML to JSON
        let schema: json::JsonValue = toml::read_file(plugin_path)?;
        let schema = json::to_string(&schema).into_diagnostic()?;

        trace!(schema = %schema, "Storing schema settings");

        manifest.config.insert("schema".to_string(), schema);
        manifest

        // Otherwise, just use the WASM plugin as is
    } else {
        debug!(source = ?plugin_path, "Loading WASM plugin");

        Tool::create_plugin_manifest(proto, Wasm::file(plugin_path))?
    };

    inject_default_manifest_config(id, proto, &mut manifest)?;

    let mut tool = Tool::load_from_manifest(id, proto, manifest)?;
    tool.locator = Some(locator.to_owned());

    Ok(tool)
}

pub async fn load_tool_with_proto(id: &Id, proto: &ProtoEnvironment) -> miette::Result<Tool> {
    let locator = locate_tool(id, &proto, false)?;

    load_tool_from_locator(id, proto, locator).await
}

pub async fn load_tool(id: &Id) -> miette::Result<Tool> {
    let proto = ProtoEnvironment::new()?;

    load_tool_with_proto(id, &proto).await
}
