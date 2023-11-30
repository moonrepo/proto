use crate::error::WarpgateError;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::{
    borrow::Borrow,
    fmt::{self, Display},
    ops::Deref,
    str::FromStr,
};

pub static ID_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new("^[a-z][a-z0-9-]*$").unwrap());

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Id(String);

impl Id {
    pub fn new<S: AsRef<str>>(id: S) -> Result<Id, WarpgateError> {
        let id = id.as_ref();

        if !ID_PATTERN.is_match(id) {
            return Err(WarpgateError::InvalidID(id.to_owned()));
        }

        Ok(Self::raw(id))
    }

    pub fn raw<S: AsRef<str>>(id: S) -> Id {
        Id(id.as_ref().to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for Id {
    fn generate_schema() -> schematic::SchemaType {
        schematic::SchemaType::string()
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<String> for Id {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

impl AsRef<Id> for Id {
    fn as_ref(&self) -> &Id {
        self
    }
}

impl Deref for Id {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Id {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == other
    }
}

impl PartialEq<String> for Id {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

// Allows strings to be used for collection keys

impl Borrow<String> for Id {
    fn borrow(&self) -> &String {
        &self.0
    }
}

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// Parsing values

impl FromStr for Id {
    type Err = WarpgateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Id::new(s)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Id::new(String::deserialize(deserializer)?)
            .map_err(|error| de::Error::custom(error.to_string()))
    }
}
