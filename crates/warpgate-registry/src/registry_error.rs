use starbase_styles::{Style, Stylize};
use thiserror::Error;
use warpgate::WarpgatePluginError;

/// Registry errors.
#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum WarpgateRegistryError {
    #[cfg_attr(feature = "miette", diagnostic(transparent))]
    #[error(transparent)]
    Plugin(#[from] Box<WarpgatePluginError>),

    #[cfg_attr(feature = "miette", diagnostic(code(registry::publish::required_func)))]
    #[error(
        "Unable to publish, required function {} is missing in WASM plugin.",
				.func.style(Style::Shell)
    )]
    PublishRequiredFunc { func: String },
}

impl From<WarpgatePluginError> for WarpgateRegistryError {
    fn from(e: WarpgatePluginError) -> WarpgateRegistryError {
        WarpgateRegistryError::Plugin(Box::new(e))
    }
}
