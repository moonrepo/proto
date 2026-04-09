use super::merge_iter;
use crate::id::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use serde::Serialize;

// `[shell]`
#[derive(Clone, Config, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProtoShellConfig {
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    #[setting(merge = merge_iter)]
    pub aliases: FxHashMap<Id, String>,
}
