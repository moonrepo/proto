use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

/// A container for WASI virtual paths that can also keep a reference to the original real path.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VirtualPath {
    /// A virtual path with prefixes to determine a real path.
    WithReal {
        path: PathBuf,
        virtual_prefix: PathBuf,
        real_prefix: PathBuf,
    },

    /// Only a virtual path.
    Only(PathBuf),
}

impl VirtualPath {
    /// Return the original real path.
    pub fn real_path(&self) -> PathBuf {
        match self {
            Self::Only(_) => panic!("No real path prefix!"),
            Self::WithReal {
                path,
                virtual_prefix,
                real_prefix,
            } => real_prefix.join(path.strip_prefix(virtual_prefix).unwrap_or(path)),
        }
    }

    /// Return a reference to the virtual path.
    pub fn virtual_path(&self) -> &PathBuf {
        match self {
            Self::Only(path) => path,
            Self::WithReal { path, .. } => path,
        }
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for VirtualPath {
    fn generate_schema() -> schematic::SchemaType {
        schematic::SchemaType::String(schematic::schema::StringType {
            format: Some("path".into()),
            ..Default::default()
        })
    }
}

impl Default for VirtualPath {
    fn default() -> Self {
        Self::Only(PathBuf::new())
    }
}

impl Deref for VirtualPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        self.virtual_path()
    }
}

impl DerefMut for VirtualPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Only(path) => path,
            Self::WithReal { path, .. } => path,
        }
    }
}

impl fmt::Display for VirtualPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.virtual_path().display())
    }
}
