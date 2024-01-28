use serde::Serialize;
use std::{borrow::Borrow, fmt, ops::Deref};

#[derive(Clone, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Id(String);

impl Id {
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

// Using as a reference

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

// Inherit string methods

impl Deref for Id {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// For comparisons

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

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Borrow<String> for Id {
    fn borrow(&self) -> &String {
        &self.0
    }
}
