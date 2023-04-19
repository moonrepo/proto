use crate::errors::ProtoError;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Debug)]
pub enum PluginLocator {
    Schema(String),
    // Source(String),
    // GitHub(String),
}

impl FromStr for PluginLocator {
    type Err = ProtoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.splitn(2, ':').map(|p| p.to_owned()).collect::<Vec<_>>();

        let Some(protocol) = parts.get(0) else {
            return Err(ProtoError::InvalidPluginLocator);
        };

        let Some(location) = parts.get(1) else {
            return Err(ProtoError::InvalidPluginLocator);
        };

        if location.is_empty() {
            return Err(ProtoError::InvalidPluginLocator);
        }

        let locator = match protocol.as_ref() {
            "schema" => {
                if !is_url_or_path(location) {
                    return Err(ProtoError::InvalidPluginLocator);
                } else if !location.ends_with(".toml") {
                    return Err(ProtoError::InvalidPluginLocatorExt(".toml".into()));
                }

                PluginLocator::Schema(location.to_owned())
            }
            other => {
                return Err(ProtoError::InvalidPluginProtocol(other.to_owned()));
            }
        };

        Ok(locator)
    }
}

fn is_url_or_path(value: &str) -> bool {
    value.starts_with("https://")
        || value.starts_with("./")
        || value.starts_with(".\\")
        || value.starts_with("..")
}

impl Serialize for PluginLocator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            PluginLocator::Schema(s) => serializer.serialize_str(&format!("schema:{}", s)),
            // PluginLocator::Source(s) => serializer.serialize_str(&format!("source:{}", s)),
            // PluginLocator::GitHub(s) => serializer.serialize_str(&format!("github:{}", s)),
        }
    }
}

impl<'de> Deserialize<'de> for PluginLocator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let locator = PluginLocator::from_str(&string).map_err(serde::de::Error::custom)?;

        Ok(locator)
    }
}
