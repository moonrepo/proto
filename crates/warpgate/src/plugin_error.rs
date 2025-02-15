use crate::id::Id;
use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

/// Plugin/runtime errors.
#[derive(Debug, Diagnostic, Error)]
pub enum WarpgatePluginError {
    #[diagnostic(code(plugin::wasm::failed_container))]
    #[error("Failed to load and create {} plugin: {error}", .id.to_string().style(Style::Id))]
    FailedContainer {
        id: Id,
        #[source]
        error: Box<extism::Error>,
    },

    #[diagnostic(code(plugin::wasm::failed_function_call))]
    #[error(
        "Failed to call {} plugin function {}:\n{error}",
        .id.to_string().style(Style::Id),
        .func.style(Style::Property),
    )]
    FailedPluginCall { id: Id, func: String, error: String },

    #[diagnostic(code(plugin::wasm::failed_function_call))]
    #[error("{error}")]
    FailedPluginCallRelease { error: String },

    #[diagnostic(code(plugin::wasm::incompatible_runtime))]
    #[error(
        "The loaded {} plugin is incompatible with the current runtime.\nFor plugin consumers, try upgrading to a newer plugin version.\nFor plugin authors, upgrade to the latest runtime and release a new version.",
        .id.to_string().style(Style::Id),
    )]
    IncompatibleRuntime { id: Id },

    #[diagnostic(code(plugin::wasm::invalid_input))]
    #[error(
        "Failed to format input for {} plugin function {} call.",
        .id.to_string().style(Style::Id),
        .func.style(Style::Property),
    )]
    InvalidInput {
        id: Id,
        func: String,
        #[source]
        error: Box<serde_json::Error>,
    },

    #[diagnostic(code(plugin::wasm::invalid_output))]
    #[error(
        "Failed to parse output of {} plugin function {} call.",
        .id.to_string().style(Style::Id),
        .func.style(Style::Property),
    )]
    InvalidOutput {
        id: Id,
        func: String,
        #[source]
        error: Box<serde_json::Error>,
    },

    #[diagnostic(code(plugin::invalid_id))]
    #[error(
        "Invalid plugin identifier {}. May only contain letters, numbers, dashes, and underscores.",
        .0.style(Style::Id),
    )]
    InvalidID(String),

    #[diagnostic(code(plugin::wasm::missing_command))]
    #[error(
        "Command or script {} does not exist. Unable to execute from plugin.", .command.style(Style::Shell)
    )]
    MissingCommand { command: String },
}
