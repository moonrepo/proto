use crate::tool_spec::Backend;
use proto_pdk_api::Checksum;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct LockfileRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<Backend>,

    // Build from source and native installs may not have a checksum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<Checksum>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl LockfileRecord {
    pub fn new(backend: Option<Backend>) -> Self {
        Self {
            backend,
            ..Default::default()
        }
    }
}
