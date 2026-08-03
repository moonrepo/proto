use crate::create_path_type;
use crate::path_utils::{PathParseError, VirtualPathShape};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::{Path, PathBuf};

create_path_type!(
    VirtualPath,
    "Represents a virtual absolute path on the guest system."
);

impl TryFrom<VirtualPathShape> for VirtualPath {
    type Error = PathParseError;

    fn try_from(value: VirtualPathShape) -> Result<Self, Self::Error> {
        match value {
            VirtualPathShape::Real(path) | VirtualPathShape::Virtual { path, .. } => Ok(Self(path)),
        }
    }
}
