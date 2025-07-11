use serde::{Deserialize, Serialize};

pub const WASM_LAYER_MEDIA_TYPE_WASM: &str = "application/wasm";
pub const WASM_LAYER_MEDIA_TYPE_TOML: &str = "application/toml";
pub const WASM_LAYER_MEDIA_TYPE_JSON: &str = "application/json";
pub const WASM_LAYER_MEDIA_TYPE_YAML: &str = "application/yaml";

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, schematic::Schematic)]
#[serde(default, rename_all = "kebab-case")]
pub struct RegistryConfig {
    pub registry: String,
    pub organization: Option<String>,
}
