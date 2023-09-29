#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("System dependency is missing a package name for the target OS and architecture.")]
    MissingName,

    #[error("No system package manager was detected.")]
    MissingPackageManager,

    #[error("Unknown or unsupported system package manager `{0}`.")]
    UnknownPackageManager(String),
}
