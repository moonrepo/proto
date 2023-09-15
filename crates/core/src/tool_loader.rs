use crate::error::ProtoError;
use crate::proto::ProtoEnvironment;
use crate::tool::Tool;
use crate::tools_config::ToolsConfig;
use crate::user_config::UserConfig;
use extism::Manifest;
use miette::IntoDiagnostic;
use proto_pdk_api::{HostArch, HostEnvironment, HostOS, UserConfigSettings};
use proto_wasm_plugin::Wasm;
use starbase_utils::{fs, json};
use std::str::FromStr;
use std::{
    env::{self, consts},
    path::Path,
};
use tracing::debug;
use warpgate::{to_virtual_path, HttpOptions, Id, PluginLocator};

pub fn inject_default_manifest_config(
    id: &Id,
    proto: &ProtoEnvironment,
    user_config: &UserConfig,
    manifest: &mut Manifest,
) -> miette::Result<()> {
    manifest
        .config
        .insert("proto_tool_id".to_string(), id.to_string());

    manifest.config.insert(
        "proto_user_config".to_string(),
        json::to_string(&UserConfigSettings {
            auto_clean: user_config.auto_clean,
            auto_install: user_config.auto_install,
            node_intercept_globals: user_config.node_intercept_globals,
        })
        .into_diagnostic()?,
    );

    manifest.config.insert(
        "proto_environment".to_string(),
        json::to_string(&HostEnvironment {
            arch: HostArch::from_str(consts::ARCH).into_diagnostic()?,
            os: HostOS::from_str(consts::OS).into_diagnostic()?,
            home_dir: to_virtual_path(manifest, &proto.home),
            proto_dir: to_virtual_path(manifest, &proto.root),
        })
        .into_diagnostic()?,
    );

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

    let plugin_loader = proto.get_plugin_loader();
    let http_client = plugin_loader.create_http_client_with_options(HttpOptions::default())?;

    let plugin_path = plugin_loader
        .load_plugin(&id, locator, &http_client)
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
                    .load_plugin(id, ToolsConfig::schema_plugin(), &http_client)
                    .await?,
            ),
        )?;

        manifest
            .config
            .insert("schema".to_string(), fs::read_file(plugin_path)?);

        manifest

        // Otherwise, just use the WASM plugin as is
    } else {
        debug!(source = ?plugin_path, "Loading WASM plugin");

        Tool::create_plugin_manifest(proto, Wasm::file(plugin_path))?
    };

    inject_default_manifest_config(id, proto, user_config, &mut manifest)?;

    Tool::load_from_manifest(id, proto, manifest)
}

pub async fn load_tool(id: &Id) -> miette::Result<Tool> {
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

    load_tool_from_locator(id, proto, locator, &user_config).await
}
