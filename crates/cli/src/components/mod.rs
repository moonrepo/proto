mod env_var;
mod locator;

pub use env_var::*;
pub use locator::*;

pub fn is_path_like(value: impl AsRef<str>) -> bool {
    let value = value.as_ref();
    value.contains('/') || value.contains("\\")
}
