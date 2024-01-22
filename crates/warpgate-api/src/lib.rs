mod host;
mod host_funcs;
mod virtual_path;

pub use anyhow::anyhow;
pub use host::*;
pub use host_funcs::*;
pub use virtual_path::*;

/// Wrap a struct with common derives and serde required attributes.
#[macro_export]
macro_rules! api_struct {
    ($struct:item) => {
        #[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
        #[serde(default)]
        $struct
    };
}

/// Wrap an enum with common derives and serde required attributes.
#[macro_export]
macro_rules! api_enum {
    ($struct:item) => {
        #[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
        $struct
    };
}

api_struct!(
    /// Represents an empty input.
    pub struct EmptyInput {}
);

pub type AnyResult<T> = anyhow::Result<T>;
