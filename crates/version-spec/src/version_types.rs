use crate::spec_error::SpecError;
use crate::{get_calver_regex, get_semver_regex};
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use std::ops::Deref;

/// Container for a semantic version.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemVer(pub Version, pub Option<String>);

impl SemVer {
    /// Parse the string into a [`semver::Version`] type.
    pub fn parse(value: &str) -> Result<Self, SpecError> {
        let Some(caps) = get_semver_regex().captures(value) else {
            return Err(SpecError::InvalidSemverFormat);
        };

        Ok(Self(
            Version::parse(caps.name("version").unwrap().as_str())?,
            caps.name("scope")
                .and_then(|cap| cap.as_str().strip_suffix('-'))
                .map(|scope| scope.to_owned()),
        ))
    }
}

impl Serialize for SemVer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for SemVer {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;

        Self::parse(&value).map_err(de::Error::custom)
    }
}

impl Deref for SemVer {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scope) = &self.1 {
            write!(f, "{scope}-")?;
        }

        write!(f, "{}", self.0)
    }
}

/// Container for a calendar version.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CalVer(pub Version, pub Option<String>);

impl CalVer {
    /// If the provided value is a calver-like version string,
    /// parse and convert it to a semver compatible version string,
    /// so that we can utilize the [`semver::Version`] type.
    pub fn parse(value: &str) -> Result<Self, SpecError> {
        let Some(caps) = get_calver_regex().captures(value) else {
            return Err(SpecError::InvalidCalverFormat);
        };

        // Short years (less than 4 characters) are relative
        // from the year 2000, so let's enforce it. Is this correct?
        // https://calver.org/#scheme
        let year = caps
            .name("year")
            .map(|cap| cap.as_str().trim_start_matches('0'))
            .unwrap_or("0");
        let mut year_no: usize = if year.is_empty() {
            0
        } else {
            year.parse().map_err(|_| SpecError::InvalidYear)?
        };

        if year.len() < 4 {
            year_no += 2000;
        }

        // Strip leading zeros from months and days. If the value is
        // not provided, fallback to a zero, as calver is 1-index based
        // and we can use this 0 for comparison.
        let month = caps
            .name("month")
            .map(|cap| cap.as_str().trim_start_matches('0'))
            .unwrap_or("0");

        let day = caps
            .name("day")
            .map(|cap| cap.as_str().trim_start_matches('0'))
            .unwrap_or("0");

        let mut version = format!("{year_no}.{month}.{day}");

        if let Some(pre) = caps.name("pre") {
            version.push_str(pre.as_str());
        }

        if let Some(micro) = caps.name("micro") {
            version.push('+');
            version.push_str(micro.as_str());
        }

        Ok(Self(
            Version::parse(&version)?,
            caps.name("scope")
                .and_then(|cap| cap.as_str().strip_suffix('-'))
                .map(|scope| scope.to_owned()),
        ))
    }
}

impl Serialize for CalVer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CalVer {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;

        Self::parse(&value).map_err(de::Error::custom)
    }
}

impl Deref for CalVer {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for CalVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scope) = &self.1 {
            write!(f, "{scope}-")?;
        }

        let version = &self.0;

        write!(f, "{:0>4}-{:0>2}", version.major, version.minor)?;

        if version.patch > 0 {
            write!(f, "-{:0>2}", version.patch)?;
        }

        // micro
        if !version.build.is_empty() {
            write!(f, ".{}", version.build)?;
        }

        if !version.pre.is_empty() {
            write!(f, "-{}", version.pre)?;
        }

        Ok(())
    }
}
