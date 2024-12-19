use crate::error::ProtoError;
use crate::proto::ProtoEnvironment;
use crate::proto_config::SCHEMA_PLUGIN_KEY;
use crate::tool::Tool;
use convert_case::{Case, Casing};
use starbase_utils::{json, toml, yaml};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, trace};
use warpgate::{inject_default_manifest_config, Id, PluginLocator, PluginManifest, Wasm};

#[instrument(skip(proto, manifest))]
pub fn inject_proto_manifest_config(
    id: &Id,
    proto: &ProtoEnvironment,
    manifest: &mut PluginManifest,
) -> miette::Result<()> {
    let config = proto.load_config()?;

    if let Some(tool_config) = config.tools.get(id) {
        let value = json::format(&tool_config.config, false)?;

        trace!(config = %value, "Storing proto tool configuration");

        manifest
            .config
            .insert("proto_tool_config".to_string(), value);
    }

    Ok(())
}

#[instrument(skip(proto))]
pub fn locate_tool(id: &Id, proto: &ProtoEnvironment) -> miette::Result<PluginLocator> {
    let mut locator = None;
    let configs = proto.load_config_manager()?;

    debug!(tool = id.as_str(), "Finding a configured plugin");

    // Check config files for plugins
    for file in &configs.files {
        if let Some(plugins) = &file.config.plugins {
            if let Some(maybe_locator) = plugins.get(id) {
                debug!(file = ?file.path, plugin = maybe_locator.to_string(), "Found a plugin");

                locator = Some(maybe_locator.to_owned());
                break;
            }
        }
    }

    // And finally the built-in plugins (must include global config)
    if locator.is_none() {
        let builtin_plugins = configs.get_merged_config()?.builtin_plugins();

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

pub async fn load_schema_plugin_with_proto(
    proto: impl AsRef<ProtoEnvironment>,
) -> miette::Result<PathBuf> {
    let proto = proto.as_ref();
    let schema_id = Id::raw(SCHEMA_PLUGIN_KEY);
    let schema_locator = locate_tool(&schema_id, proto)?;

    proto
        .get_plugin_loader()?
        .load_plugin(schema_id, schema_locator)
        .await
}

pub fn load_schema_config(plugin_path: &Path) -> miette::Result<json::JsonValue> {
    let mut is_toml = false;
    let mut schema: json::JsonValue = match plugin_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap()
    {
        "toml" => {
            is_toml = true;
            toml::read_file(plugin_path)?
        }
        "json" | "jsonc" => json::read_file(plugin_path)?,
        "yaml" | "yml" => yaml::read_file(plugin_path)?,
        _ => unimplemented!(),
    };

    // Convert object keys to kebab-case since the original
    // configuration format was based on TOML
    fn convert_config(config: &mut json::JsonValue, is_toml: bool) {
        match config {
            json::JsonValue::Array(array) => {
                for item in array {
                    convert_config(item, is_toml);
                }
            }
            json::JsonValue::Object(object) => {
                let mut map = json::JsonMap::default();

                for (key, value) in object.iter_mut() {
                    convert_config(value, is_toml);

                    map.insert(
                        if is_toml {
                            key.to_owned()
                        } else {
                            key.from_case(Case::Camel).to_case(Case::Kebab)
                        },
                        value.to_owned(),
                    );
                }

                // serde_json doesn't allow mutating keys in place,
                // so we need to rebuild the entire map...
                object.clear();
                object.extend(map);
            }
            _ => {}
        }
    }

    convert_config(&mut schema, is_toml);

    Ok(schema)
}

#[instrument(name = "load_tool", skip(proto))]
pub async fn load_tool_from_locator(
    id: impl AsRef<Id> + Debug,
    proto: impl AsRef<ProtoEnvironment>,
    locator: impl AsRef<PluginLocator> + Debug,
) -> miette::Result<Tool> {
    let id = id.as_ref();
    let proto = proto.as_ref();
    let locator = locator.as_ref();

    let plugin_path = proto.get_plugin_loader()?.load_plugin(id, locator).await?;
    let plugin_ext = plugin_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_owned());

    let mut manifest = match plugin_ext.as_deref() {
        Some("wasm") => {
            debug!(source = ?plugin_path, "Loading WASM plugin");

            Tool::create_plugin_manifest(proto, Wasm::file(plugin_path))?
        }
        Some("toml" | "json" | "jsonc" | "yaml" | "yml") => {
            debug!(format = plugin_ext, source = ?plugin_path, "Loading non-WASM plugin");

            let mut manifest = Tool::create_plugin_manifest(
                proto,
                Wasm::file(load_schema_plugin_with_proto(proto).await?),
            )?;

            let schema = json::format(&load_schema_config(&plugin_path)?, false)?;

            trace!(schema = %schema, "Storing schema settings");

            manifest.config.insert("proto_schema".to_string(), schema);
            manifest
        }
        // This case is handled by warpgate when loading the plugin
        _ => unimplemented!(),
    };

    inject_default_manifest_config(id, &proto.home_dir, &mut manifest)?;
    inject_proto_manifest_config(id, proto, &mut manifest)?;

    let mut tool = Tool::load_from_manifest(id, proto, manifest).await?;
    tool.locator = Some(locator.to_owned());

    Ok(tool)
}

pub async fn load_tool_with_proto(id: &Id, proto: &ProtoEnvironment) -> miette::Result<Tool> {
    load_tool_from_locator(id, proto, locate_tool(id, proto)?).await
}
