use crate::flow::resolve::ProtoResolveError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use tracing::warn;
use version_spec::{UnresolvedVersionSpec, VersionSpec};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(into = "String", try_from = "String")]
pub struct ToolSpec {
    /// Requested version/requirement.
    pub req: UnresolvedVersionSpec,

    /// Resolved version.
    pub version: Option<VersionSpec>,

    /// Resolve a version from the lockfile?
    pub read_lockfile: bool,

    /// Resolve a version from the manifest?
    pub resolve_from_manifest: bool,

    /// Update the lockfile when applicable?
    pub write_lockfile: bool,
}

impl ToolSpec {
    pub fn new(req: UnresolvedVersionSpec) -> Self {
        Self {
            req,
            ..Default::default()
        }
    }

    pub fn is_fully_qualified(&self) -> bool {
        matches!(
            self.req,
            UnresolvedVersionSpec::Canary
                | UnresolvedVersionSpec::Calendar(_)
                | UnresolvedVersionSpec::Semantic(_)
        )
    }

    pub fn parse<T: AsRef<str>>(value: T) -> Result<Self, ProtoResolveError> {
        Self::from_str(value.as_ref())
    }

    pub fn resolve(&mut self, res: VersionSpec) {
        self.version = Some(res);
    }

    pub fn to_resolved_spec(&self) -> VersionSpec {
        match self.version.clone() {
            Some(res) => res,
            None => self.req.to_resolved_spec(),
        }
    }
}

impl Default for ToolSpec {
    fn default() -> Self {
        Self {
            req: UnresolvedVersionSpec::default(),
            version: None,
            read_lockfile: true,
            resolve_from_manifest: true,
            write_lockfile: true,
        }
    }
}

impl FromStr for ToolSpec {
    type Err = ProtoResolveError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let spec = if let Some((_, suffix)) = value.split_once(':') {
            warn!(
                spec = value,
                "Configuring the backend within the version is no longer supported; pass it in the identifer instead"
            );

            suffix
        } else {
            value
        };

        Ok(Self {
            req: UnresolvedVersionSpec::parse(spec).map_err(|error| {
                ProtoResolveError::InvalidVersionSpec {
                    version: value.to_owned(),
                    error: Box::new(error),
                }
            })?,
            ..Default::default()
        })
    }
}

impl TryFrom<String> for ToolSpec {
    type Error = ProtoResolveError;

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
