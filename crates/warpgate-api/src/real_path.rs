use crate::create_path_type;
use crate::path_utils::{PathParseError, VirtualPathShape};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::{Path, PathBuf};

#[doc("Represents a real absolute path on the host system.")]
create_path_type!(RealPath, "RealPath");

impl TryFrom<VirtualPathShape> for RealPath {
    type Error = PathParseError;

    fn try_from(value: VirtualPathShape) -> Result<Self, Self::Error> {
        match value {
            VirtualPathShape::Real(path) => Ok(Self(path)),
            VirtualPathShape::Virtual {
                path,
                real_prefix,
                virtual_prefix,
            } => match path.strip_prefix(&virtual_prefix) {
                Ok(rel) => Ok(Self(real_prefix.join(rel))),
                Err(_) => Err(PathParseError(format!(
                    "Failed to parse path into a real path, missing compatible virtual prefixes. Path: {:?}, Virtual Prefix: {:?}, Real Prefix: {:?}",
                    path, virtual_prefix, real_prefix
                ))),
            },
        }
    }
}
