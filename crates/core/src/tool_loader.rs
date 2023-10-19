use crate::error::ProtoError;
use crate::proto::ProtoEnvironment;
use crate::tool::Tool;
use crate::tools_config::ToolsConfig;
use crate::user_config::UserConfig;
use extism::Manifest;
use miette::IntoDiagnostic;
use proto_pdk_api::{HostArch, HostEnvironment, HostOS, UserConfigSettings};
use proto_wasm_plugin::Wasm;
use starbase_utils::{json, toml};
use std::{env, path::Path};
use tracing::{debug, trace};
use warpgate::{create_http_client_with_options, to_virtual_path, Id, PluginLocator};

pub fn inject_default_manifest_config(
    id: &Id,
    proto: &ProtoEnvironment,
    user_config: &UserConfig,
    manifest: &mut Manifest,
) -> miette::Result<()> {
    trace!(id = id.as_str(), "Storing tool identifier");

    manifest
        .config
        .insert("proto_tool_id".to_string(), id.to_string());

    let value = json::to_string(&UserConfigSettings {
        auto_clean: user_config.auto_clean,
        auto_install: user_config.auto_install,
        node_intercept_globals: user_config.node_intercept_globals,
    })
    .into_diagnostic()?;

    trace!(config = %value, "Storing user configuration");

    manifest
        .config
        .insert("proto_user_config".to_string(), value);

    let value = json::to_string(&HostEnvironment {
        arch: HostArch::from_env(),
        os: HostOS::from_env(),
        home_dir: to_virtual_path(manifest, &proto.home),
        proto_dir: to_virtual_path(manifest, &proto.root),
    })
    .into_diagnostic()?;

    trace!(env = %value, "Storing proto environment");

    manifest
        .config
        .insert("proto_environment".to_string(), value);

    Ok(())
}

pub async fn load_tool_from_locator(
    id: impl AsRef<Id>,
    proto: impl AsRef<ProtoEnvironment>,
    locator: impl AsRef<PluginLocator>,
    user_config: &UserConfig,
) -> miette::Result<Tool> {
    let id = id.as_ref();
    let proto = proto.as_ref();
    let locator = locator.as_ref();

    let http_client = create_http_client_with_options(user_config.http.clone())?;
    let plugin_loader = proto.get_plugin_loader();
    let plugin_path = plugin_loader
        .load_plugin_with_client(&id, locator, &http_client)
        .await?;

    // If a TOML plugin, we need to load the WASM plugin for it,
    // wrap it, and modify the plugin manifest.
    let mut manifest = if plugin_path
        .extension()
        .map(|ext| ext == "toml")
        .unwrap_or(false)
    {
        debug!(source = ?plugin_path, "Loading TOML plugin");

        let mut manifest = Tool::create_plugin_manifest(
            proto,
            Wasm::file(
                plugin_loader
                    .load_plugin_with_client(id, ToolsConfig::schema_plugin(), &http_client)
                    .await?,
            ),
        )?;

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

    inject_default_manifest_config(id, proto, user_config, &mut manifest)?;

    let mut tool = Tool::load_from_manifest(id, proto, manifest)?;
    tool.locator = Some(locator.to_owned());

    Ok(tool)
}

pub async fn load_tool(id: &Id) -> miette::Result<Tool> {
    let proto = ProtoEnvironment::new()?;
    let user_config = proto.get_user_config()?;
    let mut locator = None;

    debug!(
        tool = id.as_str(),
        "Traversing upwards to find a configured plugin"
    );

    // Traverse upwards checking each `.prototools` for a plugin
    if let Ok(working_dir) = env::current_dir() {
        let mut current_dir: Option<&Path> = Some(&working_dir);

        while let Some(dir) = current_dir {
            // Don't traverse past the home directory
            if dir == proto.home {
                break;
            }

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

    load_tool_from_locator(id, proto, locator, &user_config).await
}
