use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum WarpgateError {
    #[error("{0}")]
    Serde(String),

    #[diagnostic(code(plugin::source::file_missing))]
    #[error("Cannot load plugin, source file {} does not exist.", .0.style(Style::Url))]
    SourceFileMissing(PathBuf),

    #[diagnostic(code(plugin::github::asset_missing))]
    #[error(
			"Cannot download plugin {} from GitHub, no applicable asset found for release {}.",
			.repo_slug.style(Style::Id),
			.tag,
		)]
    GitHubAssetMissing { repo_slug: String, tag: String },

    #[diagnostic(code(plugin::wapm::module_missing))]
    #[error(
			"Cannot download plugin {} from wamp.io, no applicable module found for release {}.",
			.package.style(Style::Id),
			.version,
		)]
    WapmModuleMissing { package: String, version: String },

    #[diagnostic(code(plugin::create::failed))]
    #[error("Failed to load and create WASM plugin.")]
    PluginCreateFailed {
        #[source]
        error: extism::Error,
    },

    #[diagnostic(code(plugin::call_func::failed))]
    #[error("Failed to call plugin function {}.", .func.style(Style::Id))]
    PluginCallFailed {
        func: String,
        #[source]
        error: extism::Error,
    },

    #[diagnostic(code(plugin::call_func::format_input))]
    #[error("Failed to format input for plugin function {} call.", .func.style(Style::Id))]
    FormatInputFailed {
        func: String,
        #[source]
        error: serde_json::Error,
    },

    #[diagnostic(code(plugin::call_func::parse_output))]
    #[error("Failed to parse output of plugin function {} call.", .func.style(Style::Id))]
    ParseOutputFailed {
        func: String,
        #[source]
        error: serde_json::Error,
    },
}
