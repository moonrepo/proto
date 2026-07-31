use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
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

/// Convert the provided real host path to a virtual guest path.
pub fn convert_to_virtual_path(
    path: impl AsRef<Path> + Debug,
    paths_list: &[(PathBuf, PathBuf)],
) -> Option<PathBuf> {
    let path = path.as_ref();

    for (host_path, guest_path) in paths_list {
        let virtual_path = if path.starts_with(guest_path) {
            path.to_owned()
        } else if let Ok(rel_path) = path.strip_prefix(host_path) {
            guest_path.join(rel_path)
        } else {
            continue;
        };

        return Some(prepare_to_path(virtual_path));
    }

    None
}

/// Convert the provided virtual guest path to a real host path.
pub fn convert_to_real_path(
    path: impl AsRef<Path> + Debug,
    paths_list: &[(PathBuf, PathBuf)],
) -> Option<PathBuf> {
    let path = path.as_ref();

    for (host_path, guest_path) in paths_list {
        let real_path = if path.starts_with(host_path) {
            path.to_owned()
        } else if let Ok(rel_path) = path.strip_prefix(guest_path) {
            host_path.join(rel_path)
        } else {
            continue;
        };

        return Some(prepare_from_path(real_path));
    }

    None
}

/// Error type for path parsing failures.
pub struct PathParseError(pub String);

impl Display for PathParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

#[macro_export]
macro_rules! create_path_type {
    ($name:ident, $label:expr) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        #[serde(try_from = "VirtualPathShape")]
        pub struct $name(PathBuf);

        impl $name {
            pub fn new(path: impl AsRef<Path>) -> Self {
                Self(path.as_ref().to_path_buf())
            }

            pub fn as_path(&self) -> &Path {
                &self.0
            }

            pub fn into_inner(self) -> PathBuf {
                self.0
            }
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
                Some($label.into())
            }

            fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
                schema.string(schematic::schema::StringType {
                    format: Some("path".into()),
                    ..Default::default()
                })
            }
        }
    };
}
