use miette::Diagnostic;
use regex::Regex;
use serde::{Deserialize, Serialize};
use starbase_styles::{Style, Stylize};
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::LazyLock;
use thiserror::Error;
use warpgate::Id;

static ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[a-zA-Z][a-zA-Z0-9-_]*$").unwrap());

#[derive(Error, Debug, Diagnostic)]
#[diagnostic(code(proto::invalid_id))]
#[error(
    "Invalid plugin identifier {}. May only contain letters, numbers, dashes, and underscores.",
    .0.style(Style::Id),
)]
pub struct ProtoIdError(String);

/// An identifier for plugins.
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

impl TryFrom<&str> for ProtoId {
    type Error = ProtoIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        ProtoId::new(value)
    }
}

impl TryFrom<String> for ProtoId {
    type Error = ProtoIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ProtoId::new(value)
    }
}

impl TryFrom<&String> for ProtoId {
    type Error = ProtoIdError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        ProtoId::new(value)
    }
}

impl schematic::Schematic for ProtoId {
    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.set_description("An identifier for plugins.");
        schema.string_default()
    }
}
