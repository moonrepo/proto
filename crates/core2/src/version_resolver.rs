use crate::version::{AliasOrVersion, VersionType};
use crate::ProtoError;
use semver::{Version, VersionReq};
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct VersionResolver {
    pub aliases: BTreeMap<String, AliasOrVersion>,
    pub versions: Vec<Version>,
}

impl VersionResolver {
    pub fn resolve<V: AsRef<str>>(&self, candidate: V) -> miette::Result<Version> {
        let candidate = candidate.as_ref();

        match VersionType::from_str(candidate)? {
            VersionType::Alias(alias) => {
                if let Some(alias_or_version) = self.aliases.get(&alias) {
                    return match alias_or_version {
                        AliasOrVersion::Alias(alias) => self.resolve(alias),
                        AliasOrVersion::Version(version) => Ok(version.to_owned()),
                    };
                }
            }
            VersionType::ReqAll(req) => {
                if let Some(version) = self.match_highest_version(req) {
                    return Ok(version);
                }
            }
            VersionType::ReqAny(reqs) => {
                for req in reqs {
                    if let Some(version) = self.match_highest_version(req) {
                        return Ok(version);
                    }
                }
            }
            VersionType::Version(ver) => {
                for version in &self.versions {
                    if &ver == version {
                        return Ok(ver);
                    }
                }
            }
        }

        Err(ProtoError::VersionResolveFailed {
            version: candidate.to_owned(),
        }
        .into())
    }

    pub fn match_highest_version(&self, req: VersionReq) -> Option<Version> {
        let mut highest_match = None;

        for version in &self.versions {
            if req.matches(version)
                && (highest_match.is_none() || highest_match.as_ref().is_some_and(|v| version > v))
            {
                highest_match = Some(version.to_owned());
            }
        }

        highest_match
    }
}
