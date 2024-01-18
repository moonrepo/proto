mod host;
mod host_funcs;
mod virtual_path;

pub use host::*;
pub use host_funcs::*;
pub use virtual_path::*;

api_struct!(
    /// Represents an empty input.
    pub struct EmptyInput {}
);

#[macro_export]
macro_rules! api_struct {
    ($struct:item) => {
        #[derive(Clone, Debug, Default, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
        #[serde(default)]
        $struct
    };
}

#[macro_export]
macro_rules! api_enum {
    ($struct:item) => {
        #[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
        $struct
    };
}
