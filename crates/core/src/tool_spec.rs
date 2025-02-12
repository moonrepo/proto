use crate::tool_error::ProtoToolError;
use schematic::{derive_enum, ConfigEnum};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use version_spec::{UnresolvedVersionSpec, VersionSpec};

derive_enum!(
    #[derive(Copy, ConfigEnum, Default, Hash)]
    pub enum Backend {
        Asdf,
        #[default]
        Proto,
    }
);

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct ToolSpec {
    pub backend: Backend,

    // Requested version/requirement
    pub req: UnresolvedVersionSpec,

    // Resolved version
    pub res: Option<VersionSpec>,
}

impl ToolSpec {
    pub fn new(req: UnresolvedVersionSpec) -> Self {
        Self {
            backend: Backend::Proto,
            req,
            res: None,
        }
    }

    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, ProtoToolError> {
        Self::from_str(value.as_ref())
    }

    pub fn resolve(&mut self, res: VersionSpec) {
        self.res = Some(res);
    }
}

impl FromStr for ToolSpec {
    type Err = ProtoToolError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (backend, spec) = if let Some((prefix, suffix)) = value.split_once(':') {
            let backend = if prefix == "proto" {
                Backend::Proto
            } else if prefix == "asdf" {
                Backend::Asdf
            } else {
                return Err(ProtoToolError::UnknownBackend {
                    backends: Backend::variants(),
                    spec: value.to_owned(),
                });
            };

            (backend, suffix)
        } else {
            (Backend::Proto, value)
        };

        Ok(Self {
            backend,
            req: UnresolvedVersionSpec::parse(spec).map_err(|error| {
                ProtoToolError::InvalidVersionSpec {
                    spec: value.to_owned(),
                    error: Box::new(error),
                }
            })?,
            res: None,
        })
    }
}

impl TryFrom<String> for ToolSpec {
    type Error = ProtoToolError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for ToolSpec {
    fn into(self) -> String {
        self.to_string()
    }
}

impl From<UnresolvedVersionSpec> for ToolSpec {
    fn from(value: UnresolvedVersionSpec) -> Self {
        Self::new(value)
    }
}

impl PartialEq<UnresolvedVersionSpec> for ToolSpec {
    fn eq(&self, other: &UnresolvedVersionSpec) -> bool {
        &self.req == other
    }
}

impl PartialEq<VersionSpec> for ToolSpec {
    fn eq(&self, other: &VersionSpec) -> bool {
        &self.req == other
    }
}

impl AsRef<ToolSpec> for ToolSpec {
    fn as_ref(&self) -> &ToolSpec {
        self
    }
}

impl AsRef<UnresolvedVersionSpec> for ToolSpec {
    fn as_ref(&self) -> &UnresolvedVersionSpec {
        &self.req
    }
}

impl fmt::Display for ToolSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.backend {
            Backend::Asdf => {
                write!(f, "asdf:")?;
            }
            Backend::Proto => {
                // No prefix
            }
        };

        write!(f, "{}", self.req)
    }
}

impl schematic::Schematic for ToolSpec {
    fn schema_name() -> Option<String> {
        Some("ToolSpec".into())
    }

    fn build_schema(mut schema: schematic::SchemaBuilder) -> schematic::Schema {
        schema.string_default()
    }
}
