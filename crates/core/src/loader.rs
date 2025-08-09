use crate::config::SCHEMA_PLUGIN_KEY;
use crate::env::ProtoEnvironment;
use crate::loader_error::ProtoLoaderError;
use crate::tool::Tool;
use crate::tool_context::ToolContext;
use convert_case::{Case, Casing};
use rustc_hash::FxHashSet;
use starbase_utils::{json, toml, yaml};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, trace, warn};
use warpgate::{Id, PluginLocator, PluginManifest, Wasm, inject_default_manifest_config};

#[instrument(skip(proto, manifest))]
pub fn inject_proto_manifest_config(
    id: &Id,
    proto: &ProtoEnvironment,
    manifest: &mut PluginManifest,
) -> Result<(), ProtoLoaderError> {
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
pub fn locate_tool(id: &Id, proto: &ProtoEnvironment) -> Result<PluginLocator, ProtoLoaderError> {
    let mut locator = None;
    let config = proto.load_config()?;

    debug!(id = id.as_str(), "Finding a configured plugin");

    // Check config files for plugins
    if let Some(maybe_locator) = config.plugins.get(id) {
        debug!(
            id = id.as_str(),
            plugin = maybe_locator.to_string(),
            "Found a plugin"
        );

        locator = Some(maybe_locator.to_owned());
    }

    // And finally the built-in plugins (must include global config)
    if locator.is_none()
        && let Some(maybe_locator) = config.builtin_plugins().get(id)
    {
        debug!(
            id = id.as_str(),
            plugin = maybe_locator.to_string(),
            "Using a built-in plugin"
        );

        locator = Some(maybe_locator.to_owned());
    }

    // Search in registries
    if locator.is_none()
        && !config.settings.registries.is_empty()
        && let Ok(maybe_locator) = PluginLocator::try_from(format!("registry://{id}"))
    {
        debug!(
            id = id.as_str(),
            plugin = maybe_locator.to_string(),
            "Using a registry plugin"
        );

        locator = Some(maybe_locator.to_owned());
    }

    let Some(mut locator) = locator else {
        return Err(ProtoLoaderError::UnknownTool { id: id.to_owned() });
    };

    // Rewrite if a URL
    if let PluginLocator::Url(inner) = &mut locator {
        inner.url = config.rewrite_url(&inner.url);
    }

    Ok(locator)
}

pub async fn load_schema_plugin_with_proto(
    proto: impl AsRef<ProtoEnvironment>,
) -> Result<PathBuf, ProtoLoaderError> {
    let proto = proto.as_ref();
    let schema_id = Id::raw(SCHEMA_PLUGIN_KEY);
    let schema_locator = locate_tool(&schema_id, proto)?;

    let path = proto
        .get_plugin_loader()?
        .load_plugin(schema_id, schema_locator)
        .await?;

    Ok(path)
}

pub fn load_schema_config(plugin_path: &Path) -> Result<json::JsonValue, ProtoLoaderError> {
    let mut is_toml = false;
    let mut schema: json::JsonValue = match plugin_path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => {
            is_toml = true;
            toml::read_file(plugin_path)?
        }
        Some("json" | "jsonc") => json::read_file(plugin_path)?,
        Some("yaml" | "yml") => yaml::read_file(plugin_path)?,
        _ => unimplemented!(),
    };

    // These are maps with user provided keys, so we shouldn't conver the casing
    let preserved_keys = FxHashSet::from_iter([
        "aliases",
        "arch",
        "exes",
        "libc",
        "platform",
        "secondary",
        "shim-env-vars",
    ]);

    // Convert object keys to kebab-case since the original
    // configuration format was based on TOML
    fn convert_config(
        config: &mut json::JsonValue,
        preserved_keys: &FxHashSet<&str>,
        parent_key: &str,
        is_toml: bool,
    ) {
        match config {
            json::JsonValue::Array(array) => {
                for item in array {
                    convert_config(item, preserved_keys, parent_key, is_toml);
                }
            }
            json::JsonValue::Object(object) => {
                let mut map = json::JsonMap::default();

                for (key, value) in object.iter_mut() {
                    let next_key = if is_toml || preserved_keys.contains(parent_key) {
                        key.to_owned()
                    } else {
                        key.from_case(Case::Camel).to_case(Case::Kebab)
                    };

                    convert_config(value, preserved_keys, &next_key, is_toml);

                    map.insert(next_key, value.to_owned());
                }

                // serde_json doesn't allow mutating keys in place,
                // so we need to rebuild the entire map...
                object.clear();
                object.extend(map);
            }
            _ => {}
        }
    }

    convert_config(&mut schema, &preserved_keys, "", is_toml);

    Ok(schema)
}

#[instrument(name = "load_tool", skip(proto))]
pub async fn load_tool_from_locator(
    context: impl AsRef<ToolContext> + Debug,
    proto: impl AsRef<ProtoEnvironment>,
    locator: impl AsRef<PluginLocator> + Debug,
) -> Result<Tool, ProtoLoaderError> {
    let context = context.as_ref();
    let proto = proto.as_ref();
    let locator = locator.as_ref();

    let plugin_path = proto
        .get_plugin_loader()?
        .load_plugin(&context.id, locator)
        .await?;
    let plugin_ext = plugin_path.extension().and_then(|ext| ext.to_str());

    let mut manifest = match plugin_ext {
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

    inject_default_manifest_config(&context.id, &proto.home_dir, &mut manifest)?;
    inject_proto_manifest_config(&context.id, proto, &mut manifest)?;

    let mut tool = Tool::load_from_manifest(context, proto, manifest).await?;
    tool.locator = Some(locator.to_owned());

    Ok(tool)
}

pub async fn load_tool(
    context: &ToolContext,
    proto: &ProtoEnvironment,
) -> Result<Tool, ProtoLoaderError> {
    // If backend is proto, use the tool's plugin,
    // otherwise use the backend plugin itself
    let locator_id = context.backend.as_ref().unwrap_or(&context.id);

    let tool = load_tool_from_locator(&context, proto, locate_tool(locator_id, proto)?).await?;

    Ok(tool)
}
