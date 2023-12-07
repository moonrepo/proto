use crate::error::ProtoError;
use crate::proto_config::ProtoToolConfig;
use crate::tool_manifest::ToolManifest;
use proto_pdk_api::LoadVersionsOutput;
use semver::{Version, VersionReq};
use std::collections::{BTreeMap, HashSet};
use version_spec::*;

#[derive(Default)]
pub struct VersionResolver<'tool> {
    pub aliases: BTreeMap<String, UnresolvedVersionSpec>,
    pub versions: Vec<Version>,

    manifest: Option<&'tool ToolManifest>,
    config: Option<&'tool ProtoToolConfig>,
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

    pub fn with_manifest(&mut self, manifest: &'tool ToolManifest) {
        self.manifest = Some(manifest);
    }

    pub fn with_config(&mut self, config: &'tool ProtoToolConfig) {
        self.config = Some(config);
    }

    pub fn resolve(&self, candidate: &UnresolvedVersionSpec) -> miette::Result<VersionSpec> {
        resolve_version(
            candidate,
            &self.versions,
            &self.aliases,
            self.manifest,
            self.config,
        )
    }

    pub fn resolve_without_manifest(
        &self,
        candidate: &UnresolvedVersionSpec,
    ) -> miette::Result<VersionSpec> {
        resolve_version(candidate, &self.versions, &self.aliases, None, None)
    }
}

pub fn match_highest_version(req: &VersionReq, versions: &[&Version]) -> Option<VersionSpec> {
    let mut highest_match: Option<Version> = None;

    for version in versions {
        if req.matches(version)
            && (highest_match.is_none() || highest_match.as_ref().is_some_and(|v| *version > v))
        {
            highest_match = Some((*version).clone());
        }
    }

    highest_match.map(VersionSpec::Version)
}

// Filter out aliases because they cannot be matched against
fn extract_installed_versions(installed: &HashSet<VersionSpec>) -> Vec<&Version> {
    installed
        .iter()
        .filter_map(|item| match item {
            VersionSpec::Version(v) => Some(v),
            _ => None,
        })
        .collect()
}

pub fn resolve_version(
    candidate: &UnresolvedVersionSpec,
    versions: &[Version],
    aliases: &BTreeMap<String, UnresolvedVersionSpec>,
    manifest: Option<&ToolManifest>,
    config: Option<&ProtoToolConfig>,
) -> miette::Result<VersionSpec> {
    let remote_versions = versions.iter().collect::<Vec<_>>();
    let installed_versions = if let Some(manifest) = manifest {
        extract_installed_versions(&manifest.installed_versions)
    } else {
        vec![]
    };

    match &candidate {
        UnresolvedVersionSpec::Canary => {
            return Ok(VersionSpec::Canary);
        }
        UnresolvedVersionSpec::Alias(alias) => {
            let mut alias_value = None;

            #[allow(deprecated)]
            if let Some(config) = config {
                alias_value = config.aliases.get(alias);
            } else if let Some(manifest) = manifest {
                alias_value = manifest.aliases.get(alias);
            }

            if alias_value.is_none() {
                alias_value = aliases.get(alias);
            }

            if let Some(value) = alias_value {
                return resolve_version(value, versions, aliases, manifest, config);
            }
        }
        UnresolvedVersionSpec::Req(req) => {
            // Check locally installed versions first
            if !installed_versions.is_empty() {
                if let Some(version) = match_highest_version(req, &installed_versions) {
                    return Ok(version);
                }
            }

            // Otherwise we'll need to download from remote
            if let Some(version) = match_highest_version(req, &remote_versions) {
                return Ok(version);
            }
        }
        UnresolvedVersionSpec::ReqAny(reqs) => {
            // Check locally installed versions first
            if !installed_versions.is_empty() {
                for req in reqs {
                    if let Some(version) = match_highest_version(req, &installed_versions) {
                        return Ok(version);
                    }
                }
            }

            // Otherwise we'll need to download from remote
            for req in reqs {
                if let Some(version) = match_highest_version(req, &remote_versions) {
                    return Ok(version);
                }
            }
        }
        UnresolvedVersionSpec::Version(ver) => {
            // Check locally installed versions first
            if installed_versions.contains(&ver) {
                return Ok(VersionSpec::Version(ver.to_owned()));
            }

            // Otherwise we'll need to download from remote
            for version in versions {
                if ver == version {
                    return Ok(VersionSpec::Version(ver.to_owned()));
                }
            }
        }
    }

    Err(ProtoError::VersionResolveFailed {
        version: candidate.to_string(),
    }
    .into())
}
