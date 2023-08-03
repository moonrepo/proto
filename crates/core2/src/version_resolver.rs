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
        resolve_version(
            &VersionType::from_str(candidate.as_ref())?,
            &self.versions.iter().collect::<Vec<_>>(),
            &self.aliases,
        )
    }
}

pub fn match_highest_version(req: &VersionReq, versions: &[&Version]) -> Option<Version> {
    let mut highest_match: Option<Version> = None;

    for version in versions {
        if req.matches(version)
            && (highest_match.is_none() || highest_match.as_ref().is_some_and(|v| *version > v))
        {
            highest_match = Some((**version).clone());
        }
    }

    highest_match
}

pub fn resolve_version(
    candidate: &VersionType,
    versions: &[&Version],
    aliases: &BTreeMap<String, AliasOrVersion>,
) -> miette::Result<Version> {
    match &candidate {
        VersionType::Alias(alias) => {
            if let Some(alias_or_version) = aliases.get(alias) {
                return match alias_or_version {
                    AliasOrVersion::Alias(alias) => {
                        resolve_version(&VersionType::Alias(alias.to_owned()), versions, aliases)
                    }
                    AliasOrVersion::Version(version) => Ok(version.to_owned()),
                };
            }
        }
        VersionType::ReqAll(req) => {
            if let Some(version) = match_highest_version(req, versions) {
                return Ok(version);
            }
        }
        VersionType::ReqAny(reqs) => {
            for req in reqs {
                if let Some(version) = match_highest_version(req, versions) {
                    return Ok(version);
                }
            }
        }
        VersionType::Version(ver) => {
            for version in versions {
                if ver == *version {
                    return Ok(ver.to_owned());
                }
            }
        }
    }

    Err(ProtoError::VersionResolveFailed {
        version: candidate.to_string(),
    }
    .into())
}
