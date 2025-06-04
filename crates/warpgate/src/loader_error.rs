use crate::client_error::WarpgateClientError;
use crate::id::Id;
use starbase_archive::ArchiveError;
use starbase_styles::{Style, Stylize};
use starbase_utils::fs::FsError;
use starbase_utils::glob::GlobError;
use starbase_utils::net::NetError;
use std::path::PathBuf;
use thiserror::Error;

/// Loader errors.
#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum WarpgateLoaderError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Archive(#[from] Box<ArchiveError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Client(#[from] Box<WarpgateClientError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Fs(#[from] Box<FsError>),

    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Glob(#[from] Box<GlobError>),

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::loader::failed_download)))]
    #[error(
        "Failed to download plugin from {}.",
        .url.style(Style::Url),
    )]
    FailedDownload {
        url: String,
        #[source]
        error: Box<NetError>,
    },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::loader::github::asset_missing))
    )]
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

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::loader::github::unknown_tag))
    )]
    #[error(
        "Cannot download {} plugin from GitHub ({}), no tag found, matched, or provided.",
        .id.to_string().style(Style::Id),
        .repo_slug.style(Style::Id),
    )]
    MissingGitHubTag { id: Id, repo_slug: String },

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::loader::file::missing)))]
    #[error(
        "Cannot load {} plugin, source file {} does not exist.",
        .id.to_string().style(Style::Id),
        .path.style(Style::Path),
    )]
    MissingSourceFile { id: Id, path: PathBuf },

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::loader::no_wasm)))]
    #[error(
        "No applicable {} file could be found in downloaded plugin {}.",
        ".wasm".style(Style::File),
        .path.style(Style::Path),
    )]
    NoWasmFound { path: PathBuf },

    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code(plugin::loader::not_found),
            help = "Please refer to the plugin's official documentation."
        )
    )]
    #[error(
        "Plugin download {} does not exist. Either this version may not be supported for your current operating system or architecture, or the URL is incorrect or malformed.",
        .url.style(Style::Url),
    )]
    NotFound { url: String },

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::offline)))]
    #[error("{message} An internet connection is required to request {}.", .url.style(Style::Url))]
    RequiredInternetConnection { message: String, url: String },

    #[cfg_attr(
        feature = "miette",
        diagnostic(code(plugin::loader::unsupported_extension))
    )]
    #[error(
        "Unsupported file extension {} for downloaded plugin {}.",
        .ext.style(Style::File),
        .path.style(Style::Path),
    )]
    UnsupportedDownloadExtension { ext: String, path: PathBuf },

    #[cfg_attr(feature = "miette", diagnostic(code(plugin::loader::unknown_type)))]
    #[error(
        "Unsure how to handle downloaded plugin {} as no file extension/type could be derived.",
        .path.style(Style::Path),
    )]
    UnknownDownloadType { path: PathBuf },
}

impl From<ArchiveError> for WarpgateLoaderError {
    fn from(e: ArchiveError) -> WarpgateLoaderError {
        WarpgateLoaderError::Archive(Box::new(e))
    }
}

impl From<WarpgateClientError> for WarpgateLoaderError {
    fn from(e: WarpgateClientError) -> WarpgateLoaderError {
        WarpgateLoaderError::Client(Box::new(e))
    }
}

impl From<FsError> for WarpgateLoaderError {
    fn from(e: FsError) -> WarpgateLoaderError {
        WarpgateLoaderError::Fs(Box::new(e))
    }
}

impl From<GlobError> for WarpgateLoaderError {
    fn from(e: GlobError) -> WarpgateLoaderError {
        WarpgateLoaderError::Glob(Box::new(e))
    }
}
