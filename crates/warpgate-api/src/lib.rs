mod host;
mod host_funcs;
mod virtual_path;
mod macros;

pub use host::*;
pub use host_funcs::*;
pub use virtual_path::*;

api_struct!(
    /// Represents an empty input.
    pub struct EmptyInput {}
);
