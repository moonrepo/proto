use crate::proto_config::ProtoToolConfig;
use crate::tool_manifest::ToolManifest;
use proto_pdk_api::LoadVersionsOutput;
use rustc_hash::FxHashSet;
use semver::{Version, VersionReq};
use std::collections::BTreeMap;
use tracing::trace;
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

        if !resolver.aliases.contains_key("latest") && !resolver.versions.is_empty() {
            resolver.aliases.insert(
                "latest".into(),
                UnresolvedVersionSpec::Version(resolver.versions[0].clone()),
            );
        }

        resolver
    }

    pub fn with_manifest(&mut self, manifest: &'tool ToolManifest) {
        self.manifest = Some(manifest);
    }

    pub fn with_config(&mut self, config: &'tool ProtoToolConfig) {
        self.config = Some(config);
    }

    pub fn resolve(&self, candidate: &UnresolvedVersionSpec) -> Option<VersionSpec> {
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
    ) -> Option<VersionSpec> {
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

    highest_match.map(VersionSpec::Semantic)
}

// Filter out aliases because they cannot be matched against
fn extract_installed_versions(installed: &FxHashSet<VersionSpec>) -> Vec<&Version> {
    installed
        .iter()
        .filter_map(|item| match item {
            VersionSpec::Semantic(v) => Some(v),
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
) -> Option<VersionSpec> {
    let remote_versions = versions.iter().collect::<Vec<_>>();
    let installed_versions = if let Some(manifest) = manifest {
        extract_installed_versions(&manifest.installed_versions)
    } else {
        vec![]
    };

    if manifest.is_some() {
        trace!(
            candidate = candidate.to_string(),
            "Resolving a version with manifest"
        );
    } else {
        trace!(
            candidate = candidate.to_string(),
            "Resolving a version without manifest"
        );
    }

    match &candidate {
        UnresolvedVersionSpec::Canary => {
            trace!("Resolved to canary");

            return Some(VersionSpec::Canary);
        }
        UnresolvedVersionSpec::Alias(alias) => {
            trace!(alias, "Found an alias, resolving further");

            let mut alias_value = None;

            if let Some(config) = config {
                alias_value = config.aliases.get(alias);
            }

            if alias_value.is_none() {
                alias_value = aliases.get(alias);
            }

            if let Some(value) = alias_value {
                trace!(
                    alias,
                    candidate = value.to_string(),
                    "Alias exists with a potential candidate"
                );

                return resolve_version(value, versions, aliases, manifest, config);
            } else {
                trace!(alias, "Alias does not exist, trying others");
            }
        }
        UnresolvedVersionSpec::Req(req) => {
            trace!(
                requirement = req.to_string(),
                "Found a requirement, resolving further"
            );

            // Check locally installed versions first
            if !installed_versions.is_empty() {
                if let Some(version) = match_highest_version(req, &installed_versions) {
                    trace!(
                        version = version.to_string(),
                        "Resolved to locally installed version"
                    );

                    return Some(version);
                }
            }

            // Otherwise we'll need to download from remote
            if let Some(version) = match_highest_version(req, &remote_versions) {
                trace!(
                    version = version.to_string(),
                    "Resolved to remote available version"
                );

                return Some(version);
            }

            trace!(
                req = req.to_string(),
                "No match for requirement, trying others"
            );
        }
        UnresolvedVersionSpec::ReqAny(reqs) => {
            let range = reqs.iter().map(|req| req.to_string()).collect::<Vec<_>>();

            trace!(
                range = ?range,
                "Found a range, resolving further"
            );

            // Check locally installed versions first
            if !installed_versions.is_empty() {
                for req in reqs {
                    if let Some(version) = match_highest_version(req, &installed_versions) {
                        trace!(
                            version = version.to_string(),
                            "Resolved to locally installed version"
                        );

                        return Some(version);
                    }
                }
            }

            // Otherwise we'll need to download from remote
            for req in reqs {
                if let Some(version) = match_highest_version(req, &remote_versions) {
                    trace!(
                        version = version.to_string(),
                        "Resolved to remote available version"
                    );

                    return Some(version);
                }
            }

            trace!(
                range = ?range,
                "No match for range, trying others",
            );
        }
        UnresolvedVersionSpec::Version(ver) => {
            let version_string = ver.to_string();

            trace!(
                version = &version_string,
                "Found an explicit version, resolving further"
            );

            // Check locally installed versions first
            if installed_versions.contains(&ver) {
                trace!(
                    version = &version_string,
                    "Resolved to locally installed version"
                );

                return Some(VersionSpec::Semantic(ver.to_owned()));
            }

            // Otherwise we'll need to download from remote
            for version in versions {
                if ver == version {
                    trace!(
                        version = &version_string,
                        "Resolved to remote available version"
                    );

                    return Some(VersionSpec::Semantic(ver.to_owned()));
                }
            }

            trace!(
                version = &version_string,
                "No match for version, trying others",
            );
        }
    }

    None
}
