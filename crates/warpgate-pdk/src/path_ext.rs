use crate::funcs::get_host_to_guest_paths;
use std::ffi::OsStr;
use warpgate_api::{
    AnyResult, RealPath, VirtualPath, anyhow, convert_to_real_path, convert_to_virtual_path,
};

/// Extension trait for [`RealPath`] to provide additional functionality.
pub trait RealPathExt {
    /// Create a [`RealPath`] from a value, based on the virtual paths configuration.
    fn create(value: impl AsRef<OsStr>) -> AnyResult<RealPath>;

    /// Convert to a [`VirtualPath`] if possible, based on the virtual paths configuration.
    fn to_virtual_path(&self) -> AnyResult<Option<VirtualPath>>;
}

impl RealPathExt for RealPath {
    fn create(value: impl AsRef<OsStr>) -> AnyResult<RealPath> {
        let value = value.as_ref();

        convert_to_real_path(value, get_host_to_guest_paths()?).ok_or_else(|| {
            anyhow!(
                "Failed to create real path for {}",
                value.to_str().unwrap_or("<unknown>")
            )
        })
    }

    fn to_virtual_path(&self) -> AnyResult<Option<VirtualPath>> {
        Ok(convert_to_virtual_path(self, get_host_to_guest_paths()?))
    }
}

/// Extension trait for [`VirtualPath`] to provide additional functionality.
pub trait VirtualPathExt {
    /// Create a [`VirtualPath`] from a value, based on the virtual paths configuration.
    fn create(value: impl AsRef<OsStr>) -> AnyResult<VirtualPath>;

    /// Convert to a [`RealPath`] if possible, based on the virtual paths configuration.
    fn to_real_path(&self) -> AnyResult<Option<RealPath>>;
}

impl VirtualPathExt for VirtualPath {
    fn create(value: impl AsRef<OsStr>) -> AnyResult<VirtualPath> {
        let value = value.as_ref();

        convert_to_virtual_path(value, get_host_to_guest_paths()?).ok_or_else(|| {
            anyhow!(
                "Failed to create virtual path for {}",
                value.to_str().unwrap_or("<unknown>")
            )
        })
    }

    fn to_real_path(&self) -> AnyResult<Option<RealPath>> {
        Ok(convert_to_real_path(self, get_host_to_guest_paths()?))
    }
}
