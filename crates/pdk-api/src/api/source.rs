use super::is_false;
use warpgate_api::{api_enum, api_struct};

api_struct!(
    /// Source code is contained in an archive.
    pub struct ArchiveSource {
        /// The URL to download the archive from.
        pub url: String,

        /// A path prefix within the archive to remove.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub prefix: Option<String>,
    }
);

api_struct!(
    /// Source code is located in a Git repository.
    pub struct GitSource {
        /// The URL of the Git remote.
        pub url: String,

        /// The branch/commit/tag to checkout.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reference: Option<String>,

        /// Include submodules during checkout.
        #[serde(default, skip_serializing_if = "is_false")]
        pub submodules: bool,
    }
);

api_enum!(
    /// The location in which source code can be acquired.
    #[serde(tag = "type", rename_all = "kebab-case")]
    pub enum SourceLocation {
        /// Downloaded from an archive.
        #[cfg_attr(feature = "schematic", schema(nested))]
        Archive(ArchiveSource),

        /// Cloned from a Git repository.
        #[cfg_attr(feature = "schematic", schema(nested))]
        Git(GitSource),
    }
);
