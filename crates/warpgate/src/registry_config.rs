use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, schematic::Schematic)]
#[serde(default, rename_all = "kebab-case")]
pub struct RegistryConfig {
    pub registry: String,
    pub organization: Option<String>,
}
