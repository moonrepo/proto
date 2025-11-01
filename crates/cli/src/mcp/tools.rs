use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, JsonSchema, Deserialize, Serialize)]
pub struct InstallToolRequest {
    /// Tool identifier/context.
    pub tool: String,

    /// Force install if the tool already exists.
    #[serde(default)]
    pub force: bool,

    /// Pin the tool to the local configuration.
    #[serde(default)]
    pub pin: bool,

    /// Version/specification to install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
}

#[derive(Default, JsonSchema, Deserialize, Serialize)]
pub struct InstallToolResponse {
    pub installed: bool,
    pub spec: String,
}

#[derive(Default, JsonSchema, Deserialize, Serialize)]
pub struct UninstallToolRequest {
    /// Tool identifier/context.
    pub tool: String,

    /// Version/specification to install.
    pub spec: String,
}

#[derive(Default, JsonSchema, Deserialize, Serialize)]
pub struct UninstallToolResponse {
    pub uninstalled: bool,
    pub spec: String,
}

#[derive(Default, JsonSchema, Deserialize, Serialize)]
pub struct ListToolVersionsRequest {
    /// Tool identifier/context.
    pub tool: String,

    /// Include all available versions, otherwise the latest 25.
    #[serde(default)]
    pub all: bool,
}

#[derive(Default, JsonSchema, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolVersionsResponse {
    pub aliases: BTreeMap<String, String>,
    pub installed_versions: Vec<String>,
    pub versions: Vec<String>,
}
