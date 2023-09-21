#[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged, into = "String", try_from = "String")]
pub enum VersionSpec {
    Alias(String),
    Version(Version),
}

impl VersionSpec {
    pub fn parse<T: AsRef<str>>(value: T) -> miette::Result<Self> {
        Ok(Self::from_str(value.as_ref())?)
    }

    pub fn is_canary(&self) -> bool {
        match self {
            Self::Alias(alias) => alias == "canary",
            Self::Version(_) => false,
        }
    }

    pub fn is_latest(&self) -> bool {
        match self {
            Self::Alias(alias) => alias == "latest",
            Self::Version(_) => false,
        }
    }

    pub fn to_unresolved_spec(&self) -> UnresolvedVersionSpec {
        match self {
            Self::Alias(alias) => UnresolvedVersionSpec::Alias(alias.to_owned()),
            Self::Version(version) => UnresolvedVersionSpec::Version(version.to_owned()),
        }
    }
}

impl Default for VersionSpec {
    fn default() -> Self {
        Self::Alias("latest".into())
    }
}

impl FromStr for VersionSpec {
    type Err = ProtoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = remove_space_after_gtlt(remove_v_prefix(value.trim().replace(".*", "")));

        if is_alias_name(&value) {
            return Ok(VersionSpec::Alias(value));
        }

        Ok(VersionSpec::Version(Version::parse(&value).map_err(
            |error| ProtoError::Semver {
                version: value,
                error,
            },
        )?))
    }
}

impl TryFrom<String> for VersionSpec {
    type Error = ProtoError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<String> for VersionSpec {
    fn into(self) -> String {
        self.to_string()
    }
}

impl Debug for VersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for VersionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alias(alias) => write!(f, "{}", alias),
            Self::Version(version) => write!(f, "{}", version),
        }
    }
}

impl PartialEq<&str> for VersionSpec {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Self::Alias(alias) => alias == other,
            Self::Version(version) => version.to_string() == *other,
        }
    }
}

impl PartialEq<Version> for VersionSpec {
    fn eq(&self, other: &Version) -> bool {
        match self {
            Self::Version(version) => version == other,
            _ => false,
        }
    }
}

#[cfg(feature = "schematic")]
impl schematic::Schematic for VersionSpec {
    fn generate_schema() -> schematic::SchemaType {
        schematic::SchemaType::string()
    }
}
