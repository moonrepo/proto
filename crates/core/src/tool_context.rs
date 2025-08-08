use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use warpgate::{Id, WarpgatePluginError};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct ToolContext {
    /// ID of the backend that tool is sourced from.
    pub backend: Option<Id>,

    /// ID of the tool itself.
    pub id: Id,
}

impl ToolContext {
    pub fn new(id: Id) -> Self {
        Self { backend: None, id }
    }

    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, WarpgatePluginError> {
        Self::from_str(value.as_ref())
    }
}

impl FromStr for ToolContext {
    type Err = WarpgatePluginError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (backend, id) = if let Some((prefix, suffix)) = value.split_once(':') {
            (Some(Id::new(prefix)?), Id::new(suffix)?)
        } else {
            (None, Id::new(value)?)
        };

        Ok(Self { backend, id })
    }
}

impl TryFrom<String> for ToolContext {
    type Error = WarpgatePluginError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

#[allow(clippy::from_over_into)]
impl Into<String> for ToolContext {
    fn into(self) -> String {
        self.to_string()
    }
}

impl AsRef<ToolContext> for ToolContext {
    fn as_ref(&self) -> &ToolContext {
        self
    }
}

impl fmt::Display for ToolContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(backend) = &self.backend {
            write!(f, "{backend}")?;
            write!(f, ":")?;
        }

        write!(f, "{}", self.id)
    }
}

impl schematic::Schematic for ToolContext {
    fn schema_name() -> Option<String> {
        Some("ToolContext".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}
