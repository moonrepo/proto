use super::{PluginType, merge_iter};
use crate::config::PROTO_PLUGIN_KEY;
use crate::id::Id;
use schematic::{Config, ValidateError, ValidateResult};
use serde::Serialize;
use std::collections::BTreeMap;
use warpgate::PluginLocator;

fn validate_reserved_words<T>(
    value: &BTreeMap<Id, PluginLocator>,
    _partial: &T,
    _context: &(),
    _finalize: bool,
) -> ValidateResult {
    if value.contains_key(PROTO_PLUGIN_KEY) {
        return Err(ValidateError::new(
            "proto is a reserved keyword, cannot use as a plugin identifier",
        ));
    }

    Ok(())
}

#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoPluginsConfig {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[setting(merge = merge_iter, validate = validate_reserved_words)]
    pub backends: BTreeMap<Id, PluginLocator>,

    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[setting(merge = merge_iter, validate = validate_reserved_words)]
    pub tools: BTreeMap<Id, PluginLocator>,

    // This is unfortunately required for flattening!
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    #[setting(merge = merge_iter, validate = validate_reserved_words)]
    pub legacy: BTreeMap<Id, PluginLocator>,
}

impl ProtoPluginsConfig {
    pub fn get(&self, id: &Id, ty: PluginType) -> Option<&PluginLocator> {
        if ty == PluginType::Backend {
            self.backends.get(id)
        } else {
            self.tools.get(id).or_else(|| self.legacy.get(id))
        }
    }
}
