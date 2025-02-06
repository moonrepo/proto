mod host;
mod host_funcs;
mod locator;
mod locator_error;
mod virtual_path;

pub use anyhow::anyhow;
pub use host::*;
pub use host_funcs::*;
pub use locator::*;
pub use locator_error::*;
pub use virtual_path::*;

/// Wrap a struct with common derives and serde required attributes.
#[macro_export]
macro_rules! api_struct {
    ($struct:item) => {
        #[derive(Clone, Debug, Default, serde::Deserialize, PartialEq, serde::Serialize)]
        #[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
        $struct
    };
}

/// Wrap an enum with common derives and serde required attributes.
#[macro_export]
macro_rules! api_enum {
    ($struct:item) => {
        #[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
        #[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
        $struct
    };
}

/// Wrap a unit-only enum with common derives and serde required attributes.
#[macro_export]
macro_rules! api_unit_enum {
    ($struct:item) => {
        #[derive(Clone, Copy, Debug, Default, serde::Deserialize, PartialEq, serde::Serialize)]
        #[serde(rename_all = "kebab-case")]
        #[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
        $struct
    };
}

api_struct!(
    /// Represents an empty input.
    pub struct EmptyInput {}
);

/// Represents any result (using `anyhow`).
pub type AnyResult<T> = anyhow::Result<T>;
