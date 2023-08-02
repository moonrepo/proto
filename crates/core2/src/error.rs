use miette::Diagnostic;
use starbase_styles::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ProtoError {
    #[diagnostic(code(proto::env::home_dir))]
    #[error("Unable to determine your home directory.")]
    MissingHomeDir,

    #[diagnostic(code(proto::version::unresolved))]
    #[error("Failed to resolve a semantic version for {}.", .version.style(Style::Hash))]
    VersionResolveFailed { version: String },

    #[diagnostic(code(proto::version::invalid))]
    #[error("Invalid version or requirement {}.", .version.style(Style::Hash))]
    Semver {
        version: String,
        #[source]
        error: semver::Error,
    },
}
