pub mod checksum;
mod config;
mod config_error;
mod env;
mod env_error;
mod file_manager;
pub mod flow;
mod helpers;
pub mod layout;
mod loader;
mod loader_error;
mod lockfile;
pub mod registry;
mod tool;
mod tool_context;
mod tool_error;
mod tool_manifest;
mod tool_spec;
pub mod utils;
mod version_resolver;

pub use config::*;
pub use config_error::*;
pub use env::*;
pub use env_error::*;
pub use file_manager::*;
pub use helpers::*;
pub use loader::*;
pub use loader_error::*;
pub use lockfile::*;
pub use tool::*;
pub use tool_context::*;
pub use tool_error::*;
pub use tool_manifest::*;
pub use tool_spec::*;
pub use version_resolver::*;
pub use version_spec::*;

// Only export things consumers will actually need!
pub use semver::{Version, VersionReq};
pub use warpgate;
pub use warpgate::{Id, PluginLocator, RegistryConfig};

// For document editing
pub mod cfg {
    pub use toml_edit::*;

    pub fn implicit_table() -> Item {
        let mut item = table();
        item.as_table_mut().unwrap().set_implicit(true);
        item
    }
}
