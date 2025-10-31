use proto_core::ToolContext;
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Deserialize, Serialize)]
pub struct InstallToolRequest {
    pub tool: String,
    pub force: bool,
    pub pin: bool,
    pub spec: Option<String>,
}

#[derive(JsonSchema, Deserialize, Serialize)]
pub struct InstallToolResponse {
    pub installed: bool,
    pub spec: String,
}
