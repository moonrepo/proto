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

impl StringOrVec {
    pub fn as_string(&self) -> String {
        match self {
            Self::String(value) => value.to_owned(),
            Self::Vec(value) => value.iter().cloned().collect::<Vec<_>>().join(" "),
        }
    }

    pub fn as_vec(&self) -> Vec<String> {
        match self {
            Self::String(value) => vec![value.to_owned()],
            Self::Vec(value) => value.to_owned(),
        }
    }
}
