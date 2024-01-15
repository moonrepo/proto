mod virtual_path;

pub use virtual_path::*;

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
