use super::{EnvFile, EnvVar, merge_iter};
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use schematic::{Config, merge};
use serde::Serialize;
use starbase_utils::json::JsonValue;

// `[backends.id]`
#[derive(Clone, Config, Debug, Serialize)]
#[config(allow_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoBackendConfig {
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    #[setting(nested, merge = merge_iter)]
    pub env: IndexMap<String, EnvVar>,

    // Custom configuration to pass to plugins
    #[serde(flatten, skip_serializing_if = "FxHashMap::is_empty")]
    #[setting(merge = merge_iter)]
    pub config: FxHashMap<String, JsonValue>,

    #[serde(skip)]
    #[setting(exclude, merge = merge::append_vec)]
    pub(crate) _env_files: Vec<EnvFile>,
}
