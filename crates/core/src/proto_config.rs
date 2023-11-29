use serde::{Deserialize, Serialize};
use starbase_utils::toml::TomlValue;
use std::collections::BTreeMap;
use version_spec::*;
use warpgate::{HttpOptions, Id, PluginLocator};

pub const PROTO_CONFIG_NAME: &str = ".prototools";
pub const SCHEMA_PLUGIN_KEY: &str = "internal-schema";

fn is_empty<K, V>(map: &BTreeMap<K, V>) -> bool {
    map.is_empty()
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DetectStrategy {
    #[default]
    FirstAvailable,
    PreferPrototools,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PinType {
    Global,
    Local,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ProtoToolConfig {
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,

    // Custom configuration to pass to plugins
    #[serde(flatten, skip_serializing_if = "is_empty")]
    pub config: BTreeMap<String, TomlValue>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ProtoSettingsConfig {
    pub auto_clean: bool,
    pub auto_install: bool,
    pub detect_strategy: DetectStrategy,
    pub pin_latest: Option<PinType>,
    pub http: HttpOptions,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ProtoConfig {
    #[serde(skip_serializing_if = "is_empty")]
    pub plugins: BTreeMap<Id, PluginLocator>,

    pub settings: ProtoSettingsConfig,

    #[serde(skip_serializing_if = "is_empty")]
    pub tools: BTreeMap<Id, ProtoToolConfig>,

    #[serde(flatten, skip_serializing_if = "is_empty")]
    pub versions: BTreeMap<Id, UnresolvedVersionSpec>,
}
