use crate::id::Id;
use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum WarpgateError {
    #[diagnostic(code(plugin::invalid_syntax))]
    #[error("{0}")]
    Serde(String),

    #[diagnostic(code(plugin::http))]
    #[error("Failed to make HTTP request for {}.", .url.style(Style::Url))]
    Http {
        url: String,
        #[source]
        error: Box<reqwest::Error>,
    },

    #[diagnostic(code(plugin::offline))]
    #[error("{message} An internet connection is required to request {}.", .url.style(Style::Url))]
    InternetConnectionRequired { message: String, url: String },

    #[diagnostic(code(plugin::invalid_id))]
    #[error(
        "Invalid plugin identifier {}. May only contain letters, numbers, dashes, and underscores.",
        .0.style(Style::Id),
    )]
    InvalidID(String),

    #[diagnostic(code(plugin::source::file_missing))]
    #[error(
        "Cannot load {} plugin, source file {} does not exist.",
        .id.style(Style::Id),
        .path.style(Style::Path),
    )]
    SourceFileMissing { id: Id, path: PathBuf },

    #[diagnostic(code(plugin::github::asset_missing))]
    #[error(
        "Cannot download {} plugin from GitHub ({}), no tag found, matched, or provided.",
        .id.style(Style::Id),
        .repo_slug.style(Style::Id),
    )]
    GitHubTagMissing { id: Id, repo_slug: String },

    #[diagnostic(code(plugin::github::asset_missing))]
    #[error(
        "Cannot download {} plugin from GitHub ({}), no applicable asset found for release {}.",
        .id.style(Style::Id),
        .repo_slug.style(Style::Id),
        .tag,
    )]
    GitHubAssetMissing {
        id: Id,
        repo_slug: String,
        tag: String,
    },

    #[diagnostic(code(plugin::create::failed))]
    #[error("Failed to load and create {} plugin: {error}", .id.style(Style::Id))]
    PluginCreateFailed {
        id: Id,
        #[source]
        error: Box<extism::Error>,
    },

    #[diagnostic(code(plugin::call_func::failed))]
    #[error(
        "Failed to call {} plugin function {}:\n{error}",
        .id.style(Style::Id),
        .func.style(Style::Property),
    )]
    PluginCallFailed { id: Id, func: String, error: String },

    #[diagnostic(code(plugin::call_func::failed))]
    #[error("{error}")]
    PluginCallFailedRelease { error: String },

    #[diagnostic(code(plugin::missing_command))]
    #[error(
        "Command or script {} does not exist. Unable to execute from plugin.", .command.style(Style::Shell)
    )]
    PluginCommandMissing { command: String },

    #[diagnostic(code(plugin::call_func::format_input))]
    #[error(
        "Failed to format input for {} plugin function {} call.",
        .id.style(Style::Id),
        .func.style(Style::Property),
    )]
    FormatInputFailed {
        id: Id,
        func: String,
        #[source]
        error: Box<serde_json::Error>,
    },

    #[diagnostic(code(plugin::call_func::parse_output))]
    #[error(
        "Failed to parse output of {} plugin function {} call.",
        .id.style(Style::Id),
        .func.style(Style::Property),
    )]
    ParseOutputFailed {
        id: Id,
        func: String,
        #[source]
        error: Box<serde_json::Error>,
    },

    #[diagnostic(
        code(plugin::download::not_found),
        help = "Please refer to the plugin's official documentation."
    )]
    #[error(
        "Plugin download {} does not exist. Either this version may not be supported for your current operating system or architecture, or the URL is incorrect or malformed.",
        .url.style(Style::Url),
    )]
    DownloadNotFound { url: String },

    #[diagnostic(code(plugin::download::no_wasm))]
    #[error(
        "No applicable {} file could be found in downloaded plugin {}.",
        ".wasm".style(Style::File),
        .path.style(Style::Path),
    )]
    DownloadNoWasm { path: PathBuf },

    #[diagnostic(code(plugin::download::unsupported_extension))]
    #[error(
        "Unsupported file extension {} for downloaded plugin {}.",
        .ext.style(Style::File),
        .path.style(Style::Path),
    )]
    DownloadUnsupportedExtension { ext: String, path: PathBuf },

    #[diagnostic(code(plugin::download::unknown_type))]
    #[error(
        "Unsure how to handle downloaded plugin {} as no file extension/type could be derived.",
        .path.style(Style::Path),
    )]
    DownloadUnknownType { path: PathBuf },

    #[diagnostic(code(plugin::incompatible_runtime))]
    #[error(
        "The loaded {} plugin is incompatible with the current runtime.\nFor plugin consumers, try upgrading to a newer plugin version.\nFor plugin authors, upgrade to the latest runtime and release a new version.",
        .id.style(Style::Id),
    )]
    IncompatibleRuntime { id: Id },
}
