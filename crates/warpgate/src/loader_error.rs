use crate::id::Id;
use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum WarpgateLoaderError {
    #[diagnostic(code(plugin::loader::github::asset_missing))]
    #[error(
        "Cannot download {} plugin from GitHub ({}), no applicable asset found for release {}.",
        .id.to_string().style(Style::Id),
        .repo_slug.style(Style::Id),
        .tag,
    )]
    MissingGitHubAsset {
        id: Id,
        repo_slug: String,
        tag: String,
    },

    #[diagnostic(code(plugin::loader::github::unknown_tag))]
    #[error(
        "Cannot download {} plugin from GitHub ({}), no tag found, matched, or provided.",
        .id.to_string().style(Style::Id),
        .repo_slug.style(Style::Id),
    )]
    MissingGitHubTag { id: Id, repo_slug: String },

    #[diagnostic(code(plugin::loader::file::missing))]
    #[error(
        "Cannot load {} plugin, source file {} does not exist.",
        .id.to_string().style(Style::Id),
        .path.style(Style::Path),
    )]
    MissingSourceFile { id: Id, path: PathBuf },

    #[diagnostic(code(plugin::loader::no_wasm))]
    #[error(
        "No applicable {} file could be found in downloaded plugin {}.",
        ".wasm".style(Style::File),
        .path.style(Style::Path),
    )]
    NoWasmFound { path: PathBuf },

    #[diagnostic(
        code(plugin::loader::not_found),
        help = "Please refer to the plugin's official documentation."
    )]
    #[error(
        "Plugin download {} does not exist. Either this version may not be supported for your current operating system or architecture, or the URL is incorrect or malformed.",
        .url.style(Style::Url),
    )]
    NotFound { url: String },

    #[diagnostic(code(plugin::offline))]
    #[error("{message} An internet connection is required to request {}.", .url.style(Style::Url))]
    RequiredInternetConnection { message: String, url: String },

    #[diagnostic(code(plugin::loader::unsupported_extension))]
    #[error(
        "Unsupported file extension {} for downloaded plugin {}.",
        .ext.style(Style::File),
        .path.style(Style::Path),
    )]
    UnsupportedDownloadExtension { ext: String, path: PathBuf },

    #[diagnostic(code(plugin::loader::unknown_type))]
    #[error(
        "Unsure how to handle downloaded plugin {} as no file extension/type could be derived.",
        .path.style(Style::Path),
    )]
    UnknownDownloadType { path: PathBuf },
}
