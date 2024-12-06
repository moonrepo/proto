mod aliases_map;
mod env_var;
mod locator;
mod versions_map;

pub use aliases_map::*;
pub use env_var::*;
pub use locator::*;
pub use versions_map::*;

pub fn is_path_like(value: impl AsRef<str>) -> bool {
    let value = value.as_ref();
    value.contains('/') || value.contains("\\")
}
