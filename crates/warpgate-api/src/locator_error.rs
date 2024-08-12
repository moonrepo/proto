/// Errors during plugin locator parsing.
#[derive(thiserror::Error, Debug)]
pub enum PluginLocatorError {
    #[error("GitHub release locator requires a repository with organization scope (org/repo).")]
    GitHubMissingOrg,

    #[error("Missing plugin location (after protocol).")]
    MissingLocation,

    #[error("Missing plugin protocol.")]
    MissingProtocol,

    #[error("Only https URLs are supported for plugins.")]
    SecureUrlsOnly,

    #[error("Unknown plugin protocol `{0}`.")]
    UnknownProtocol(String),
}
