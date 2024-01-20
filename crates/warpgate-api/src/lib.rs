mod host;
mod host_funcs;
mod macros;
mod virtual_path;

#[cfg(feature = "pdk")]
pub mod pdk;

pub use anyhow::anyhow;
pub use host::*;
pub use host_funcs::*;
pub use virtual_path::*;

api_struct!(
    /// Represents an empty input.
    pub struct EmptyInput {}
);
