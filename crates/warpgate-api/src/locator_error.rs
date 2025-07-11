/// Errors during plugin locator parsing.
#[derive(thiserror::Error, Debug)]
pub enum PluginLocatorError {
    #[error("GitHub release locator requires a repository name with owner scope (owner/repo).")]
    MissingGitHubOrg,

    #[error("Missing plugin location (after protocol).")]
    MissingLocation,

    #[error("Missing plugin protocol. Supports file://, https://, and github://.")]
    MissingProtocol,

    #[error("Only secure URLs (https://) are supported for plugins.")]
    SecureUrlsOnly,

    #[error("Missing registry image.")]
    MissingRegistryImage,

    #[error("Unknown plugin protocol `{0}`.")]
    UnknownProtocol(String),
}
