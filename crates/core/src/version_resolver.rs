use crate::error::ProtoError;
use crate::tool_manifest::ToolManifest;
use crate::version::UnresolvedVersionSpec;
use crate::VersionSpec;
use proto_pdk_api::LoadVersionsOutput;
use semver::{Version, VersionReq};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Default)]
pub struct VersionResolver<'tool> {
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,
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
                .insert(alias, UnresolvedVersionSpec::Version(version));
        }

        if let Some(latest) = output.latest {
            resolver
                .aliases
                .insert("latest".into(), UnresolvedVersionSpec::Version(latest));
        }

        // Sort from newest to oldest
        resolver.versions.sort_by(|a, d| d.cmp(a));

        resolver
    }

    pub fn with_manifest(&mut self, manifest: &'tool ToolManifest) -> miette::Result<()> {
        self.manifest = Some(manifest);

        Ok(())
    }

    pub fn resolve(&self, candidate: &UnresolvedVersionSpec) -> miette::Result<Version> {
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

// Filter out aliases because they cannot be matched against
fn extract_installed_versions(installed: &HashSet<VersionSpec>) -> Vec<&Version> {
    installed
        .iter()
        .filter_map(|item| match item {
            VersionSpec::Alias(_) => None,
            VersionSpec::Version(v) => Some(v),
        })
        .collect()
}

pub fn resolve_version(
    candidate: &UnresolvedVersionSpec,
    versions: &[Version],
    aliases: &BTreeMap<String, UnresolvedVersionSpec>,
    manifest: Option<&ToolManifest>,
) -> miette::Result<Version> {
    match &candidate {
        UnresolvedVersionSpec::Alias(alias) => {
            if let Some(manifest) = manifest {
                if let Some(alias_type) = manifest.aliases.get(alias) {
                    return resolve_version(alias_type, versions, aliases, Some(manifest));
                }
            }

            if let Some(alias_type) = aliases.get(alias) {
                return resolve_version(alias_type, versions, aliases, manifest);
            }
        }
        UnresolvedVersionSpec::Req(req) => {
            if let Some(manifest) = manifest {
                if let Some(version) = match_highest_version(
                    req,
                    extract_installed_versions(&manifest.installed_versions),
                ) {
                    return Ok(version);
                }
            }

            if let Some(version) = match_highest_version(req, versions) {
                return Ok(version);
            }
        }
        UnresolvedVersionSpec::ReqAny(reqs) => {
            for req in reqs {
                if let Some(manifest) = manifest {
                    if let Some(version) = match_highest_version(
                        req,
                        extract_installed_versions(&manifest.installed_versions),
                    ) {
                        return Ok(version);
                    }
                }

                if let Some(version) = match_highest_version(req, versions) {
                    return Ok(version);
                }
            }
        }
        UnresolvedVersionSpec::Version(ver) => {
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
