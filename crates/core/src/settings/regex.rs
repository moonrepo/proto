use schematic::{Schema, SchemaBuilder, Schematic};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct RegexSetting(pub regex::Regex);

impl Deref for RegexSetting {
    type Target = regex::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<String> for RegexSetting {
    type Error = regex::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(regex::Regex::new(&value)?))
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for RegexSetting {
    fn into(self) -> String {
        self.to_string()
    }
}

impl PartialEq<RegexSetting> for RegexSetting {
    fn eq(&self, other: &RegexSetting) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for RegexSetting {}

impl Hash for RegexSetting {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.0.as_str().as_bytes());
    }
}

impl Schematic for RegexSetting {
    fn build_schema(_: SchemaBuilder) -> Schema {
        SchemaBuilder::generate::<regex::Regex>()
    }
}
