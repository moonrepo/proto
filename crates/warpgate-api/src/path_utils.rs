use crate::real_path::RealPath;
use crate::virtual_path::VirtualPath;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::{Path, PathBuf};

#[cfg(any(unix, target_os = "wasi"))]
pub(crate) fn prepare_to_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().to_path_buf()
}

#[cfg(any(unix, target_os = "wasi"))]
pub(crate) fn prepare_from_path(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().to_path_buf()
}

// Only forward slashes are allowed in WASI. This is also required
// when joining paths in WASM, because mismatched separators will
// cause issues.

#[cfg(windows)]
pub(crate) fn prepare_to_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(path.as_ref().to_string_lossy().replace('\\', "/"))
}

#[cfg(windows)]
pub(crate) fn prepare_from_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(path.as_ref().to_string_lossy().replace('/', "\\"))
}

/// Sort paths from longest to shortest host path,
/// so that prefix replacing is deterministic and accurate.
pub fn sort_paths_list(paths_list: &mut [(PathBuf, PathBuf)]) {
    paths_list.sort_by(|a, d| d.0.cmp(&a.0).then(d.1.cmp(&a.1)));
}

// When the path being converted is the prefix itself, the stripped
// relative path is empty, and `Path::join("")` would append a trailing
// separator, so return the base prefix untouched instead.
fn join_rel_path(base: &Path, rel_path: &Path) -> PathBuf {
    if rel_path.as_os_str().is_empty() {
        base.to_owned()
    } else {
        base.join(rel_path)
    }
}

/// Convert the provided real host path to a virtual guest path.
/// If the host path does not match any of the provided virtual paths,
/// it will return `None`.
pub fn convert_to_virtual_path(
    path: impl AsRef<Path>,
    paths_list: &[(PathBuf, PathBuf)],
) -> Option<VirtualPath> {
    let path = path.as_ref();

    for (host_path, guest_path) in paths_list {
        let virtual_path = if path.starts_with(guest_path) {
            path.to_owned()
        } else if let Ok(rel_path) = path.strip_prefix(host_path) {
            join_rel_path(guest_path, rel_path)
        } else {
            continue;
        };

        return Some(VirtualPath::new(prepare_to_path(virtual_path)));
    }

    None
}

/// Convert the provided virtual guest path to a real host path.
/// If the guest path does not match any of the provided virtual paths,
/// it will return `None`.
pub fn convert_to_real_path(
    path: impl AsRef<Path>,
    paths_list: &[(PathBuf, PathBuf)],
) -> Option<RealPath> {
    let path = path.as_ref();

    for (host_path, guest_path) in paths_list {
        let real_path = if path.starts_with(host_path) {
            path.to_owned()
        } else if let Ok(rel_path) = path.strip_prefix(guest_path) {
            join_rel_path(host_path, rel_path)
        } else {
            continue;
        };

        return Some(RealPath::new(prepare_from_path(real_path)));
    }

    None
}

/// Convert the provided virtual guest path to a real host path,
/// returning a [`PathBuf`] instead of a [`RealPath`]. If the guest path
/// does not match any of the provided virtual paths,
/// it will return the original path.
pub fn convert_to_real_native_path(
    path: impl AsRef<Path>,
    paths_list: &[(PathBuf, PathBuf)],
) -> PathBuf {
    let path = path.as_ref();

    for (host_path, guest_path) in paths_list {
        let real_path = if path.starts_with(host_path) {
            path.to_owned()
        } else if let Ok(rel_path) = path.strip_prefix(guest_path) {
            join_rel_path(host_path, rel_path)
        } else {
            continue;
        };

        return prepare_from_path(real_path);
    }

    path.to_path_buf()
}

/// Error type for path parsing failures.
pub struct PathParseError(pub String);

impl Display for PathParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Intermediate (de)serialization shape for [`VirtualPath`] and [`RealPath`].
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum VirtualPathShape {
    Virtual {
        path: PathBuf,

        #[serde(alias = "v")]
        virtual_prefix: PathBuf,

        #[serde(alias = "r")]
        real_prefix: PathBuf,
    },

    Real(PathBuf),
}

#[doc(hidden)]
#[macro_export]
macro_rules! extend_path_method {
    ($method:ident) => {
        $crate::extend_path_method!($method, std::ffi::OsStr);
    };
    ($method:ident, $ty:path) => {
        #[doc = concat!(
            "Like [`Path::", stringify!($method), "`] but returns `Self` instead of a path reference."
        )]
        pub fn $method<S>(&self, value: S) -> Self
        where
            S: AsRef<$ty>,
        {
            Self(self.0.$method(value))
        }
    };
}

/// Create a path type (real or virtual) with common trait implementations.
#[doc(hidden)]
#[macro_export]
macro_rules! create_path_type {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(try_from = "VirtualPathShape")]
        pub struct $name(PathBuf);

        impl $name {
            /// Create a new instance from the provided path.
            /// This does NOT validate the path.
            pub fn new(path: impl AsRef<std::ffi::OsStr>) -> Self {
                Self(PathBuf::from(path.as_ref()))
            }

            /// Return true if the inner path is empty.
            pub fn is_empty(&self) -> bool {
                self.0.as_os_str().is_empty()
            }

            /// Return the inner path as a [`PathBuf`].
            pub fn into_inner(self) -> PathBuf {
                self.0
            }

            /// Like [`Path::parent`] but returns `Self` instead of a path reference.
            pub fn parent(&self) -> Option<Self> {
                if self
                    .0
                    .to_str()
                    .is_some_and(|comp| comp.is_empty() || comp == "/")
                {
                    None
                } else {
                    self.0.parent().map(|p| Self(p.to_path_buf()))
                }
            }

            $crate::extend_path_method!(join, Path);
            $crate::extend_path_method!(with_added_extension);
            $crate::extend_path_method!(with_extension);
            $crate::extend_path_method!(with_file_name);
        }

        impl Default for $name {
            fn default() -> Self {
                Self(PathBuf::new())
            }
        }

        impl AsRef<$name> for $name {
            fn as_ref(&self) -> &$name {
                self
            }
        }

        impl AsRef<PathBuf> for $name {
            fn as_ref(&self) -> &PathBuf {
                &self.0
            }
        }

        impl AsRef<Path> for $name {
            fn as_ref(&self) -> &Path {
                &self.0
            }
        }

        impl AsRef<std::ffi::OsStr> for $name {
            fn as_ref(&self) -> &std::ffi::OsStr {
                self.0.as_os_str()
            }
        }

        impl std::ops::Deref for $name {
            type Target = PathBuf;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl AsMut<PathBuf> for $name {
            fn as_mut(&mut self) -> &mut PathBuf {
                &mut self.0
            }
        }

        impl AsMut<Path> for $name {
            fn as_mut(&mut self) -> &mut Path {
                &mut self.0
            }
        }

        impl AsMut<std::ffi::OsStr> for $name {
            fn as_mut(&mut self) -> &mut std::ffi::OsStr {
                self.0.as_mut_os_str()
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0.display())
            }
        }

        #[cfg(feature = "schematic")]
        impl schematic::Schematic for $name {
            fn schema_name() -> Option<String> {
                Some(stringify!($name).into())
            }

            fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
                schema.set_description($doc);
                schema.string(schematic::schema::StringType {
                    format: Some("path".into()),
                    ..Default::default()
                })
            }
        }
    };
}
