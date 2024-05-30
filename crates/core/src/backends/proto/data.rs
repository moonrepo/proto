use schematic::Schematic;
use serde::{Deserialize, Serialize};
use url::Url;
use warpgate::{Id, PluginLocator};

/// Format of the plugin.
#[derive(Deserialize, Serialize, Schematic)]
#[serde(rename_all = "lowercase")]
pub enum PluginFormat {
    Toml,
    Wasm,
}

/// Information about a person.
#[derive(Default, Deserialize, Serialize, Schematic)]
#[serde(default)]
pub struct PluginPerson {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Url>,
}

/// Information about an author, either their name, or an object of additional fields.
#[derive(Deserialize, Serialize, Schematic)]
#[serde(untagged)]
pub enum PluginAuthor {
    String(String),
    Object(PluginPerson),
}

/// A file source where the plugin attempts to detect a version from.
#[derive(Default, Deserialize, Serialize, Schematic)]
#[serde(default)]
pub struct PluginDetectionSource {
    file: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Url>,
}

/// Information about a plugin.
#[derive(Deserialize, Serialize, Schematic)]
#[serde(rename_all = "camelCase")]
pub struct PluginEntry {
    // PLUGIN
    /// Suggested identifier of the plugin. This will become the primary binary/shim name,
    /// as well as the name used on the command line, and within environment variables.
    id: Id,

    /// The location in which to acquire the plugin.
    /// More information: https://moonrepo.dev/docs/guides/wasm-plugins#configuring-plugin-locations
    locator: PluginLocator,

    /// Format of the plugin: WASM, or TOML
    format: PluginFormat,

    // METADATA
    /// Human readable name of the tool.
    name: String,

    /// Description of the tool in which the plugin is providing.
    description: String,

    /// Information about the author.
    #[schema(nested)]
    author: PluginAuthor,

    /// URL to the tool's homepage or documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    homepage_url: Option<Url>,

    /// URL to the plugin's repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    repository_url: Option<Url>,

    /// Devicon (https://devicon.dev) for the tool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    devicon: Option<String>,

    // PROVIDES
    /// List of binary/shim names that are provided by this plugin.
    bins: Vec<String>,

    /// List of sources in which versions are detected from.
    #[schema(nested)]
    #[serde(default)]
    detection_sources: Vec<PluginDetectionSource>,

    /// List of directories in which the plugin locates globally installed binaries/packages. Supports environment variables.
    #[serde(default)]
    globals_dirs: Vec<String>,
}

/// A collection of plugins that can be utilized by consumers.
#[derive(Deserialize, Serialize, Schematic)]
pub struct PluginRegistryDocument {
    /// Path to a JSON schema.
    #[serde(rename = "$schema")]
    schema: String,

    /// Current version of the registry document.
    version: u8,

    /// List of available plugins.
    #[schema(nested)]
    plugins: Vec<PluginEntry>,
}
