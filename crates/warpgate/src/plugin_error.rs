use starbase_styles::{Style, Stylize, apply_style_tags};
use thiserror::Error;
use warpgate_api::Id;

/// Plugin/runtime errors.
#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum WarpgatePluginError {
    #[cfg_attr(feature = "miette", diagnostic(code(plugin::wasm::failed_container)))]
    #[error("Failed to load and create {} plugin: {error}", .id.to_string().style(Style::Id))]
    FailedContainer {
        id: Id,
        #[source]
        error: Box<extism::Error>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::wasm::failed_function_call))
    )]
    #[error(
        "Failed to call {} plugin function {}:\n{}",
        .id.to_string().style(Style::Id),
        .func.style(Style::Property),
        apply_style_tags(.error),
    )]
    FailedPluginCall { id: Id, func: String, error: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::wasm::failed_function_call))
    )]
    #[error("{}", apply_style_tags(.error))]
    FailedPluginCallRelease { error: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::wasm::foreign_platform_command))
    )]
    #[error(
        "Unable to execute command {} from plugin, as it's targeting {} instead of the host machine. Commands only reflect the host, so they can't be used to inspect another platform.",
        .command.style(Style::Shell),
        .platform,
    )]
    ForeignPlatformCommand { command: String, platform: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::wasm::incompatible_runtime))
    )]
    #[error(
        "The loaded {} plugin is incompatible with the current runtime.\nFor plugin consumers, try upgrading to a newer plugin version.\nFor plugin authors, upgrade to the latest runtime and release a new version.",
        .id.to_string().style(Style::Id),
    )]
    IncompatibleRuntime { id: Id },

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::wasm::invalid_input)))]
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

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::wasm::invalid_output)))]
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

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::wasm::missing_command)))]
    #[error(
        "Command or script {} does not exist. Unable to execute from plugin.", .command.style(Style::Shell)
    )]
    MissingCommand { command: String },
}
