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

api_enum!(
    /// Either a boolean representing on or off, or a string representing on with a message.
    #[serde(untagged)]
    pub enum Switch {
        Toggle(bool),
        Message(String),
    }
);

impl Default for Switch {
    fn default() -> Self {
        Self::Toggle(false)
    }
}

impl Switch {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Toggle(value) => *value,
            Self::Message(_) => true,
        }
    }
}
