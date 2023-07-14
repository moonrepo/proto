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
}
