use crate::plugin_error::WarpgatePluginError;
use compact_str::CompactString;
use regex::Regex;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::sync::LazyLock;
use std::{borrow::Borrow, fmt, ops::Deref, str::FromStr};

#[doc(hidden)]
pub static ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[a-zA-Z][a-zA-Z0-9-_]*$").unwrap());

/// An identifier for plugins.
#[derive(Clone, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Id(CompactString);

impl Id {
    pub fn new<S: AsRef<str>>(id: S) -> Result<Id, WarpgatePluginError> {
        let id = id.as_ref();

        if !ID_PATTERN.is_match(id) {
            return Err(WarpgatePluginError::InvalidID(id.to_owned()));
        }

        Ok(Self::raw(id))
    }

    pub fn raw<S: AsRef<str>>(id: S) -> Id {
        Id(CompactString::new(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for Id {
    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.set_description("An identifier for plugins.");
        schema.string_default()
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// impl AsRef<String> for Id {
//     fn as_ref(&self) -> &String {
//         &self.0
//     }
// }

impl AsRef<Id> for Id {
    fn as_ref(&self) -> &Id {
        self
    }
}

impl Deref for Id {
    type Target = str;

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
        self.0 == other
    }
}

impl PartialEq<String> for Id {
    fn eq(&self, other: &String) -> bool {
        self.0 == other
    }
}

// Allows strings to be used for collection keys

// impl Borrow<String> for Id {
//     fn borrow(&self) -> &String {
//         &self.0
//     }
// }

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

// Parsing values

impl FromStr for Id {
    type Err = WarpgatePluginError;

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
