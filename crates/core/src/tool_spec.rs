use crate::tool_error::ProtoToolError;
use schematic::ConfigEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use version_spec::UnresolvedVersionSpec;

#[derive(Clone, Copy, ConfigEnum, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ToolBackend {
    Asdf,
    Proto,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct ToolSpec {
    pub backend: ToolBackend,
    pub spec: UnresolvedVersionSpec,
}

impl ToolSpec {
    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, ProtoToolError> {
        Self::from_str(value.as_ref())
    }
}

impl FromStr for ToolSpec {
    type Err = ProtoToolError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (backend, spec) = if let Some((prefix, suffix)) = value.split_once(':') {
            let backend = if prefix == "proto" {
                ToolBackend::Proto
            } else if prefix == "asdf" {
                ToolBackend::Asdf
            } else {
                return Err(ProtoToolError::UnknownBackend {
                    backends: ToolBackend::variants(),
                    spec: value.to_owned(),
                });
            };

            (backend, suffix)
        } else {
            (ToolBackend::Proto, value)
        };

        Ok(Self {
            backend,
            spec: UnresolvedVersionSpec::parse(spec).map_err(|error| {
                ProtoToolError::InvalidVersionSpec {
                    spec: value.to_owned(),
                    error: Box::new(error),
                }
            })?,
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

impl fmt::Display for ToolSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.backend {
            ToolBackend::Asdf => {
                write!(f, "asdf:")?;
            }
            ToolBackend::Proto => {
                // No prefix
            }
        };

        write!(f, "{}", self.spec)
    }
}
