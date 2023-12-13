use serde::{Deserialize, Serialize};

#[doc(hidden)]
#[macro_export]
macro_rules! json_struct {
    ($struct:item) => {
        #[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(default)]
        $struct
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! json_enum {
    ($struct:item) => {
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        $struct
    };
}

json_enum!(
    pub enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }
);
