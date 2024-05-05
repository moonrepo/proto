use warpgate_api::api_enum;

api_enum!(
    /// Either a string, or a list of strings.
    #[serde(untagged)]
    pub enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }
);

impl StringOrVec {
    pub fn as_string(&self) -> String {
        match self {
            Self::String(value) => value.to_owned(),
            Self::Vec(value) => value.to_vec().join(" "),
        }
    }
}
