use crate::funcs::get_host_to_guest_paths;
use warpgate_api::{
    AnyResult, RealPath, VirtualPath, convert_to_real_path, convert_to_virtual_path,
};

/// Extension trait for `RealPath` to provide additional functionality.
pub trait RealPathExt {
    /// Convert to a `VirtualPath` if possible, based on the virtual paths configuration.
    fn to_virtual_path(&self) -> AnyResult<Option<VirtualPath>>;
}

impl RealPathExt for RealPath {
    fn to_virtual_path(&self) -> AnyResult<Option<VirtualPath>> {
        Ok(convert_to_virtual_path(&self, get_host_to_guest_paths()?).map(VirtualPath::new))
    }
}

/// Extension trait for `VirtualPath` to provide additional functionality.
pub trait VirtualPathExt {
    /// Convert to a `RealPath` if possible, based on the virtual paths configuration.
    fn to_real_path(&self) -> AnyResult<Option<RealPath>>;
}

impl VirtualPathExt for VirtualPath {
    fn to_real_path(&self) -> AnyResult<Option<RealPath>> {
        Ok(convert_to_real_path(&self, get_host_to_guest_paths()?).map(RealPath::new))
    }
}
