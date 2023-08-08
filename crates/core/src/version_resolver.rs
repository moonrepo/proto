use crate::error::ProtoError;
use crate::tool_manifest::ToolManifest;
use crate::version::VersionType;
use proto_pdk_api::LoadVersionsOutput;
use semver::{Version, VersionReq};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct VersionResolver<'tool> {
    pub aliases: BTreeMap<String, VersionType>,
    pub versions: Vec<Version>,

    manifest: Option<&'tool ToolManifest>,
}

impl<'tool> VersionResolver<'tool> {
    pub fn from_output(output: LoadVersionsOutput) -> Self {
        let mut resolver = Self::default();
        resolver.versions.extend(output.versions);

        for (alias, version) in output.aliases {
            resolver
                .aliases
                .insert(alias, VersionType::Version(version));
        }

        if let Some(latest) = output.latest {
            resolver
                .aliases
                .insert("latest".into(), VersionType::Version(latest));
        }

        // Sort from newest to oldest
        resolver.versions.sort_by(|a, d| d.cmp(a));
        // resolver.canary_versions.sort_by(|a, d| d.cmp(a));

        resolver
    }

    pub fn with_manifest(&mut self, manifest: &'tool ToolManifest) -> miette::Result<()> {
        self.manifest = Some(manifest);

        Ok(())
    }

    pub fn resolve(&self, candidate: &VersionType) -> miette::Result<Version> {
        resolve_version(candidate, &self.versions, &self.aliases, self.manifest)
    }
}

pub fn match_highest_version<'l, I>(req: &'l VersionReq, versions: I) -> Option<Version>
where
    I: IntoIterator<Item = &'l Version>,
{
    let mut highest_match: Option<Version> = None;

    for version in versions {
        if req.matches(version)
            && (highest_match.is_none() || highest_match.as_ref().is_some_and(|v| version > v))
        {
            highest_match = Some((*version).clone());
        }
    }

    highest_match
}

pub fn resolve_version(
    candidate: &VersionType,
    versions: &[Version],
    aliases: &BTreeMap<String, VersionType>,
    manifest: Option<&ToolManifest>,
) -> miette::Result<Version> {
    match &candidate {
        VersionType::Alias(alias) => {
            if let Some(alias_type) = aliases.get(alias) {
                return resolve_version(alias_type, versions, aliases, manifest);
            }
        }
        VersionType::ReqAll(req) => {
            // Prefer installed versions
            if let Some(manifest) = manifest {
                if let Some(version) = match_highest_version(req, &manifest.installed_versions) {
                    return Ok(version);
                }
            }

            if let Some(version) = match_highest_version(req, versions) {
                return Ok(version);
            }
        }
        VersionType::ReqAny(reqs) => {
            for req in reqs {
                // Prefer installed versions
                if let Some(manifest) = manifest {
                    if let Some(version) = match_highest_version(req, &manifest.installed_versions)
                    {
                        return Ok(version);
                    }
                }

                if let Some(version) = match_highest_version(req, versions) {
                    return Ok(version);
                }
            }
        }
        VersionType::Version(ver) => {
            for version in versions {
                if ver == version {
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
