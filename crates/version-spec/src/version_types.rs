use crate::get_calver_regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

#[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SemVer(pub Version);

impl SemVer {
    pub fn parse(value: &str) -> Result<Self, semver::Error> {
        Ok(Self(Version::parse(value)?))
    }
}

impl Deref for SemVer {
    type Target = Version;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CalVer(pub Version);

impl CalVer {
    /// If the provided value is a calver-like version string,
    /// parse and convert it to a semver compatible version string,
    /// so that we can utilize the [`semver::Version`] type.
    ///
    /// If the calendar version contains a micro field, it will
    /// converted into build metadata (prefixed with +).
    pub fn parse(value: &str) -> Result<Self, semver::Error> {
        let caps = get_calver_regex().captures(&value).unwrap();

        let mut version = String::from(
            caps.name("year")
                .map(|cap| cap.as_str().trim_start_matches('0'))
                .unwrap_or("0"),
        );

        version.push('.');
        version.push_str(
            caps.name("month")
                .map(|cap| cap.as_str().trim_start_matches('0'))
                .unwrap_or("0"),
        );

        version.push('.');
        version.push_str(
            caps.name("day")
                .map(|cap| cap.as_str().trim_start_matches('0'))
                .unwrap_or("0"),
        );

        if let Some(pre) = caps.name("pre") {
            version.push_str(pre.as_str());
        }

        if let Some(micro) = caps.name("micro") {
            version.push('+');
            version.push_str(micro.as_str());
        }

        Ok(Self(Version::parse(&version)?))
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
        let version = &self.0;

        write!(f, "{}", version.major)?;

        if version.minor > 0 {
            write!(f, "-{}", version.minor)?;

            if version.patch > 0 {
                write!(f, "-{}", version.patch)?;
            }
        }

        // micro
        if !version.build.is_empty() {
            write!(f, ".{}", version.build)?;
        }

        if !version.pre.is_empty() {
            write!(f, ".{}", version.pre)?;
        }

        Ok(())
    }
}
