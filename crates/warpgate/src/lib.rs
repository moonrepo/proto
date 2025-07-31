mod client;
mod client_error;
mod helpers;
pub mod host;
mod id;
mod loader;
mod loader_error;
mod plugin;
mod plugin_error;
mod protocols;
mod registry_config;
pub mod test_utils;

pub use client::*;
pub use client_error::*;
pub use helpers::*;
pub use id::*;
pub use loader::*;
pub use loader_error::*;
pub use plugin::*;
pub use plugin_error::*;
pub use registry_config::*;

pub use extism::{Manifest as PluginManifest, Wasm};
pub use warpgate_api as api;
pub use warpgate_api::{
    FileLocator, GitHubLocator, PluginLocator, PluginLocatorError, RegistryLocator, UrlLocator,
    VirtualPath,
};
