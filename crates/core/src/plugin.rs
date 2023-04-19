use crate::errors::ProtoError;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Debug)]
pub enum PluginLocator {
    Schema(String),
    Source(String),
    GitHub(String),
}

impl FromStr for PluginLocator {
    type Err = ProtoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("github:") {
            Ok(PluginLocator::GitHub(s[7..].to_owned()))
        } else if s.starts_with("source:") {
            Ok(PluginLocator::Source(s[7..].to_owned()))
        } else if s.starts_with("schema:") {
            Ok(PluginLocator::Schema(s[7..].to_owned()))
        } else {
            let parts = s.splitn(2, ':').collect::<Vec<_>>();

            Err(ProtoError::InvalidPluginProtocol(
                parts.get(0).map(|p| (*p).to_owned()).unwrap_or_default(),
            ))
        }
    }
}

impl Serialize for PluginLocator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PluginLocator::Schema(s) => serializer.serialize_str(&format!("schema:{}", s)),
            PluginLocator::Source(s) => serializer.serialize_str(&format!("source:{}", s)),
            PluginLocator::GitHub(s) => serializer.serialize_str(&format!("github:{}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for PluginLocator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let r = PluginLocator::from_str(&s).map_err(serde::de::Error::custom)?;

        Ok(r)
    }
}
