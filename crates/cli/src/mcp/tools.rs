use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Deserialize, Serialize)]
pub struct InstallToolRequest {
    pub tool: String,
    pub force: bool,
    pub pin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
}

#[derive(JsonSchema, Deserialize, Serialize)]
pub struct InstallToolResponse {
    pub installed: bool,
    pub spec: String,
}
