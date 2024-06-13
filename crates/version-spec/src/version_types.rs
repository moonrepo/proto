use crate::get_calver_regex;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CalVer(pub Version);

impl CalVer {
    /// If the provided value is a calver-like version string,
    /// parse and convert it to a semver compatible version string,
    /// so that we can utilize the [`semver::Version`] type.
    ///
    /// If the calendar version contains a micro field, it will
    /// converted into build metadata (prefixed with +).
    pub fn parse(value: &str) -> Result<Self, semver::Error> {
        let Some(caps) = get_calver_regex().captures(value) else {
            // Attempt to parse as-is to generate an error
            return Ok(Self(Version::parse(value)?));
        };

        // Short years (less than 4 characters) are relative
        // from the year 2000, so let's enforce it. Is this correct?
        // https://calver.org/#scheme
        let year = caps
            .name("year")
            .map(|cap| cap.as_str().trim_start_matches('0'))
            .unwrap_or("0");
        let mut year_no: usize = year.parse().unwrap();

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
