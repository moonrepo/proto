use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

macro_rules! inherit_methods {
    (comparator, [$($method:ident),+ $(,)?]) => {
        $(
            #[doc = concat!("Inherited from [`Path::", stringify!($method), "`].")]
            pub fn $method(&self, value: impl AsRef<Path>) -> bool {
                self.any_path().$method(value)
            }
        )*
    };
    (getter, [$($method:ident),+ $(,)?]) => {
        $(
            #[doc = concat!("Inherited from [`Path::", stringify!($method), "`].")]
            pub fn $method(&self) -> Option<&OsStr> {
                self.any_path().$method()
            }
        )*
    };
    (setter, [$($method:ident),+ $(,)?]) => {
        $(
            #[doc = concat!("Inherited from [`PathBuf::", stringify!($method), "`].")]
            pub fn $method(&mut self, value: impl AsRef<OsStr>) {
                let path = match self {
                    Self::Real(base) => base,
                    Self::Virtual { path: base, .. } => base,
                };

                path.$method(value);
            }
        )*
    };
    ([$($method:ident),+ $(,)?]) => {
        $(
            #[doc = concat!("Inherited from [`Path::", stringify!($method), "`].")]
            pub fn $method(&self) -> bool {
                self.any_path().$method()
            }
        )*
    };
}

/// A container for WASI virtual paths that can also keep a reference to the original real path.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(untagged)]
pub enum VirtualPath {
    /// A virtual path with prefixes to determine a real path.
    Virtual {
        path: PathBuf,

        #[serde(alias = "v")]
        virtual_prefix: PathBuf,

        #[serde(alias = "r")]
        real_prefix: PathBuf,
    },

    /// Only a real path. Could not be matched with a virtual prefix.
    Real(PathBuf),
}

impl VirtualPath {
    inherit_methods!([exists, has_root, is_absolute, is_dir, is_file, is_relative]);
    inherit_methods!(getter, [extension, file_name, file_stem]);
    inherit_methods!(setter, [set_extension, set_file_name]);
    inherit_methods!(comparator, [ends_with, starts_with]);

    /// Append the path part and return a new [`VirtualPath`] instance.
    pub fn join<P: AsRef<Path>>(&self, path: P) -> VirtualPath {
        match self {
            Self::Real(base) => Self::Real(base.join(path.as_ref())),
            Self::Virtual {
                path: base,
                virtual_prefix,
                real_prefix,
            } => Self::Virtual {
                path: base.join(path.as_ref()),
                virtual_prefix: virtual_prefix.clone(),
                real_prefix: real_prefix.clone(),
            },
        }
    }

    /// Return the parent directory as a new [`VirtualPath`] instance.
    pub fn parent(&self) -> Option<VirtualPath> {
        // If at the root (`/`), then we have gone outside the allowed
        // virtual paths, so there's no parent to use!
        fn is_root(path: &Path) -> bool {
            path.to_str()
                .is_some_and(|comp| comp.is_empty() || comp == "/")
        }

        match self {
            Self::Real(base) => base.parent().and_then(|parent| {
                if is_root(parent) {
                    None
                } else {
                    Some(Self::Real(parent.to_owned()))
                }
            }),
            Self::Virtual {
                path: base,
                virtual_prefix,
                real_prefix,
            } => base.parent().and_then(|parent| {
                if is_root(parent) {
                    None
                } else {
                    Some(Self::Virtual {
                        path: parent.to_owned(),
                        virtual_prefix: virtual_prefix.clone(),
                        real_prefix: real_prefix.clone(),
                    })
                }
            }),
        }
    }

    /// Return any path available, either virtual or real, regardless of any
    /// conditions. This is primarily used for debugging.
    pub fn any_path(&self) -> &PathBuf {
        match self {
            Self::Real(path) => path,
            Self::Virtual { path, .. } => path,
        }
    }

    /// Return the original real path. If we don't have access to prefixes,
    /// or removing prefix fails, returns `None`.
    pub fn real_path(&self) -> Option<PathBuf> {
        match self {
            Self::Real(path) => Some(path.to_path_buf()),
            Self::Virtual { real_prefix, .. } => {
                self.without_prefix().map(|path| real_prefix.join(path))
            }
        }
    }

    /// Return the original real path as a string.
    pub fn real_path_string(&self) -> Option<String> {
        self.real_path()
            .and_then(|path| path.to_str().map(|path| path.to_owned()))
    }

    /// Convert the virtual path into a [`PathBuf`] instance. This *does not*
    /// convert it into a real path.
    pub fn to_path_buf(&self) -> PathBuf {
        self.any_path().to_path_buf()
    }

    /// Return the virtual path. If a real path only, returns `None`.
    pub fn virtual_path(&self) -> Option<PathBuf> {
        match self {
            Self::Real(_) => None,
            Self::Virtual { path, .. } => Some(path.to_owned()),
        }
    }

    /// Return the virtual path as a string.
    pub fn virtual_path_string(&self) -> Option<String> {
        self.virtual_path()
            .and_then(|path| path.to_str().map(|path| path.to_owned()))
    }

    /// Return the current path without a virtual prefix.
    /// If we don't have access to prefixes, returns `None`.
    pub fn without_prefix(&self) -> Option<&Path> {
        match self {
            Self::Real(_) => None,
            Self::Virtual {
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
        schema.set_description("A container for WASI virtual paths that can also keep a reference to the original real path.");
        schema.string(schematic::schema::StringType {
            format: Some("path".into()),
            ..Default::default()
        })
    }
}

impl Default for VirtualPath {
    fn default() -> Self {
        Self::Real(PathBuf::new())
    }
}

impl fmt::Display for VirtualPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.any_path().display())
    }
}

impl AsRef<VirtualPath> for VirtualPath {
    fn as_ref(&self) -> &VirtualPath {
        self
    }
}

impl AsRef<PathBuf> for VirtualPath {
    fn as_ref(&self) -> &PathBuf {
        self.any_path()
    }
}

impl AsRef<Path> for VirtualPath {
    fn as_ref(&self) -> &Path {
        self.any_path()
    }
}

impl AsRef<OsStr> for VirtualPath {
    fn as_ref(&self) -> &OsStr {
        self.any_path().as_os_str()
    }
}
