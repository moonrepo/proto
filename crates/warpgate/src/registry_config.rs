use serde::{Deserialize, Serialize};

pub const WASM_LAYER_MEDIA_TYPE_WASM: &str = "application/wasm";
pub const WASM_LAYER_MEDIA_TYPE_TOML: &str = "application/toml";
pub const WASM_LAYER_MEDIA_TYPE_JSON: &str = "application/json";
pub const WASM_LAYER_MEDIA_TYPE_YAML: &str = "application/yaml";

/// Configures an individual plugin registry.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, rename_all = "kebab-case")]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
pub struct RegistryConfig {
    /// The domain/host of the registry.
    pub registry: String,

    /// An optional namespace to bucket the plugin into.
    pub namespace: Option<String>,
}

impl RegistryConfig {
    /// Return a fully-qualified reference with the provided ID.
    pub fn get_reference(&self, id: &str) -> String {
        let mut reference = String::new();
        reference.push_str(&self.registry);
        reference.push('/');

        if let Some(namespace) = &self.namespace {
            reference.push_str(namespace);
            reference.push('/');
        }

        reference.push_str(id);
        reference
    }

    /// Return a fully-qualified reference with the provided ID and tag.
    pub fn get_reference_with_tag(&self, id: &str, tag: &str) -> String {
        let mut reference = self.get_reference(id);
        reference.push(':');
        reference.push_str(tag);
        reference
    }
}
