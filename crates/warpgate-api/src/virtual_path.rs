use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

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
    /// Append the path part and return a new [`VirtualPath`] instance.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> VirtualPath {
        match self {
            Self::Only(base) => Self::Only(base.join(path.as_ref())),
            Self::WithReal {
                path: base,
                virtual_prefix,
                real_prefix,
            } => Self::WithReal {
                path: base.join(path.as_ref()),
                virtual_prefix: virtual_prefix.clone(),
                real_prefix: real_prefix.clone(),
            },
        }
    }

    /// Return the original real path. If we don't have access to prefixes,
    /// or removing prefix fails, returns `None`.
    pub fn real_path(&self) -> Option<PathBuf> {
        match self {
            Self::Only(_) => None,
            Self::WithReal { real_prefix, .. } => {
                self.without_prefix().map(|path| real_prefix.join(path))
            }
        }
    }

    /// Return a reference to the virtual path.
    pub fn virtual_path(&self) -> &Path {
        match self {
            Self::Only(path) => path,
            Self::WithReal { path, .. } => path,
        }
    }

    /// Return the current path without a virtual prefix.
    /// If we don't have access to prefixes, returns `None`.
    pub fn without_prefix(&self) -> Option<&Path> {
        match self {
            Self::Only(_) => None,
            Self::WithReal {
                path,
                virtual_prefix,
                ..
            } => path.strip_prefix(virtual_prefix).ok(),
        }
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for VirtualPath {
    fn schema_name() -> Option<String> {
        Some("VirtualPath".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string(schematic::schema::StringType {
            format: Some("path".into()),
            ..Default::default()
        });
        schema.build()
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
        match self {
            Self::Only(path) => path,
            Self::WithReal { path, .. } => path,
        }
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

impl AsRef<VirtualPath> for VirtualPath {
    fn as_ref(&self) -> &VirtualPath {
        self
    }
}

impl AsRef<Path> for VirtualPath {
    fn as_ref(&self) -> &Path {
        self.virtual_path()
    }
}
