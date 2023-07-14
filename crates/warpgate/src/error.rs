use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum WarpgateError {
    #[diagnostic(code(plugin::source::file_missing))]
    #[error("Cannot load plugin, source file {} does not exist.", .0.style(Style::Url))]
    SourceFileMissing(PathBuf),

    #[diagnostic(code(plugin::github::asset_missing))]
    #[error(
			"Cannot download plugin from {} on GitHub, no applicable asset found for release {}.",
			.repo_slug.style(Style::Id),
			.release,
		)]
    GitHubAssetMissing { repo_slug: String, release: String },

    #[diagnostic(code(plugin::wapm::module_missing))]
    #[error(
			"Cannot download plugin from {} on wamp.io, no applicable module found for release {}.",
			.package.style(Style::Id),
			.release,
		)]
    WapmModuleMissing { package: String, release: String },
}
