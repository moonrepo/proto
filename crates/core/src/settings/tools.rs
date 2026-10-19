use super::{EnvFile, EnvVar, merge_iter, merge_unique_vec};
use crate::tool_context::ToolContext;
use crate::tool_spec::ToolSpec;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use schematic::{Config, merge};
use serde::Serialize;
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;
use warpgate::PluginLocator;

// `[tools.id]`
// `[tools."backend:tool"]`
#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoToolConfig {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[setting(merge = merge_iter)]
    pub aliases: BTreeMap<String, ToolSpec>,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    #[setting(nested, merge = merge_iter)]
    pub env: IndexMap<String, EnvVar>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<PluginLocator>,

    // Other tools that this tool depends on, in addition
    // to those required by the tool's plugin
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[setting(merge = merge_unique_vec)]
    pub requires: Vec<ToolContext>,

    // Custom configuration to pass to plugins
    #[serde(flatten, skip_serializing_if = "FxHashMap::is_empty")]
    #[setting(merge = merge_iter)]
    pub config: FxHashMap<String, JsonValue>,

    #[serde(skip)]
    #[setting(exclude, merge = merge::append_vec)]
    pub(crate) _env_files: Vec<EnvFile>,
}
