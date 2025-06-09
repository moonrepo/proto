use crate::tool_error::ProtoToolError;
use schematic::{ConfigEnum, derive_enum};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use version_spec::{UnresolvedVersionSpec, VersionSpec};

derive_enum!(
    #[derive(Copy, ConfigEnum, Hash)]
    pub enum Backend {
        Asdf,
    }
);

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct ToolSpec {
    pub backend: Option<Backend>,

    // Requested version/requirement
    pub req: UnresolvedVersionSpec,

    // Resolved version
    pub res: Option<VersionSpec>,
}

impl ToolSpec {
    pub fn new(req: UnresolvedVersionSpec) -> Self {
        Self {
            backend: None,
            req,
            res: None,
        }
    }

    pub fn new_backend(req: UnresolvedVersionSpec, backend: Option<Backend>) -> Self {
        Self {
            backend,
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

    pub fn to_resolved_spec(&self) -> VersionSpec {
        match self.res.clone() {
            Some(res) => res,
            None => self.req.to_resolved_spec(),
        }
    }
}

impl FromStr for ToolSpec {
    type Err = ProtoToolError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (backend, spec) = if let Some((prefix, suffix)) = value.split_once(':') {
            let backend = if prefix == "proto" {
                None
            } else if prefix == "asdf" {
                Some(Backend::Asdf)
            } else {
                return Err(ProtoToolError::UnknownBackend {
                    backends: Backend::variants(),
                    spec: value.to_owned(),
                });
            };

            (backend, suffix)
        } else {
            (None, value)
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

#[allow(clippy::from_over_into)]
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
        if let Some(backend) = self.backend {
            match backend {
                Backend::Asdf => {
                    write!(f, "asdf:")?;
                }
            };
        }

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
