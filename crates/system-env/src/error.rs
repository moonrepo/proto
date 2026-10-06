#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("System dependency is missing a package name for the target OS and architecture.")]
    MissingName,

    #[error("No system package manager was detected.")]
    MissingPackageManager,

    #[error("A system package manager is required for this operation.")]
    RequiredPackageManager,

    #[error(
        "Invalid platform `{0}`, expected the format `<arch>-<os>`, `<arch>-<os>-<libc>`, or a Rust target triple."
    )]
    InvalidPlatform(String),

    #[error("Unknown {kind} `{value}` in platform `{platform}`.")]
    UnknownPlatformPart {
        kind: String,
        value: String,
        platform: String,
    },

    #[error("Unknown or unsupported system package manager `{0}`.")]
    UnknownPackageManager(String),
}
