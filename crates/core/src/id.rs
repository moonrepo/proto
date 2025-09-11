use miette::Diagnostic;
use regex::Regex;
use serde::{Deserialize, Serialize};
use starbase_styles::{Style, Stylize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::LazyLock;
use thiserror::Error;

// The shared identifier.
pub use warpgate::Id;

// Identifiers are typically alphanumeric characters, dashes, and underscores,
// as they are used in directory and file names. However, we also need to support
// npm/cargo/etc packages, so we expand the list of valid characters just a bit.
static ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^(@[a-zA-Z][a-zA-Z0-9-_]*/)?[a-zA-Z][a-zA-Z0-9-_]*$").unwrap());

#[derive(Error, Debug, Diagnostic)]
#[diagnostic(code(proto::invalid_id))]
#[error(
    "Invalid plugin identifier {}. May only contain letters, numbers, dashes, and underscores.",
    .0.style(Style::Id),
)]
pub struct ProtoIdError(String);

/// An identifier that ensures that it has been formatted correctly.
/// Primarily used in configuration and serde flows.
#[derive(Clone, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct ProtoId(Id);

impl ProtoId {
    pub fn new<S: AsRef<str>>(id: S) -> Result<Self, ProtoIdError> {
        let id = id.as_ref();

        if !ID_PATTERN.is_match(id) {
            return Err(ProtoIdError(id.to_owned()));
        }

        Ok(ProtoId(Id::raw(id)))
    }

    pub fn format<S: AsRef<str>>(id: S) -> Result<Id, ProtoIdError> {
        Self::new(id).map(|id| id.into_id())
    }

    pub fn as_id(&self) -> &Id {
        &self.0
    }

    pub fn to_id(&self) -> Id {
        self.0.clone()
    }

    pub fn into_id(self) -> Id {
        self.0
    }
}

impl fmt::Debug for ProtoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for ProtoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for ProtoId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Id> for ProtoId {
    fn as_ref(&self) -> &Id {
        &self.0
    }
}

impl From<ProtoId> for String {
    fn from(value: ProtoId) -> Self {
        value.to_string()
    }
}

impl FromStr for ProtoId {
    type Err = ProtoIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        ProtoId::new(value)
    }
}

macro_rules! gen_try_from {
    ($ty:ty) => {
        impl TryFrom<$ty> for ProtoId {
            type Error = ProtoIdError;

            fn try_from(value: $ty) -> Result<Self, Self::Error> {
                ProtoId::new(value)
            }
        }
    };
}

gen_try_from!(&str);
gen_try_from!(String);
gen_try_from!(&String);

impl schematic::Schematic for ProtoId {
    fn schema_name() -> Option<String> {
        Some("Id".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}
